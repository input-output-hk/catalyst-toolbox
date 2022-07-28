use super::{Rewards, Threshold, VoteCount};
use jormungandr_lib::crypto::account::Identifier;
use rust_decimal::Decimal;
use snapshot_lib::{SnapshotInfo, VotingGroup};
use std::collections::{BTreeMap, HashSet};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("Value overflowed its maximum value")]
    Overflow,
    #[error("Multiple snapshot entries per voter are not supported")]
    MultipleEntries,
}

fn filter_requirements(
    mut dreps: Vec<SnapshotInfo>,
    votes: VoteCount,
    top_dreps_to_reward: usize,
    votes_threshold: Threshold,
) -> Vec<SnapshotInfo> {
    // only the top `top_dreps_to_reward` representatives get rewards
    dreps.sort_by_key(|v| v.hir.voting_power);
    dreps.reverse();
    dreps.truncate(top_dreps_to_reward);

    dreps
        .into_iter()
        .filter_map(|d| votes.get(&d.hir.voting_key).map(|d_votes| (d, d_votes)))
        .filter(|(_d, d_votes)| votes_threshold.filter(d_votes))
        .map(|(d, _d_votes)| d)
        .collect()
}

pub fn calc_dreps_rewards(
    snapshot: Vec<SnapshotInfo>,
    votes: VoteCount,
    drep_voting_group: VotingGroup,
    top_dreps_to_reward: usize,
    dreps_votes_threshold: Threshold,
    total_rewards: Decimal,
) -> Result<BTreeMap<Identifier, Rewards>, Error> {
    let total_active_stake = snapshot
        .iter()
        .try_fold(0u64, |acc, x| acc.checked_add(x.hir.voting_power.into()))
        .ok_or(Error::Overflow)?;

    let dreps = snapshot
        .into_iter()
        .filter(|v| v.hir.voting_group == drep_voting_group)
        .collect::<Vec<_>>();

    let unique_dreps = dreps
        .iter()
        .map(|s| s.hir.voting_key.clone())
        .collect::<HashSet<_>>();
    if unique_dreps.len() != dreps.len() {
        return Err(Error::MultipleEntries);
    }

    let filtered = filter_requirements(dreps, votes, top_dreps_to_reward, dreps_votes_threshold);

    Ok(filtered
        .into_iter()
        .map(|d| {
            let reward = Decimal::from(u64::from(d.hir.voting_power))
                / Decimal::from(total_active_stake)
                * total_rewards;
            (d.hir.voting_key, reward)
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::utils::assert_are_close;
    use fraction::Fraction;
    use jormungandr_lib::crypto::{account::Identifier, hash::Hash};
    use snapshot_lib::registration::*;
    use snapshot_lib::*;
    use std::collections::HashMap;
    use test_strategy::proptest;

    #[proptest]
    fn test_all_active(snapshot: Snapshot) {
        let votes_count = snapshot
            .voting_keys()
            .into_iter()
            .map(|key| (key.clone(), HashSet::from([Hash::from([0u8; 32])])))
            .collect::<VoteCount>();
        let n_voters = votes_count.len();
        let voters = snapshot.to_full_snapshot_info();
        let rewards = calc_dreps_rewards(
            votes_count,
            voters,
            Threshold::new(1, HashMap::new(), Vec::new()).unwrap(),
            Rewards::ONE,
        )
        .unwrap();
        if n_voters > 0 {
            assert_are_close(rewards.values().sum::<Rewards>(), Rewards::ONE)
        } else {
            assert_eq!(rewards.len(), 0);
        }
    }

    #[proptest]
    fn test_all_inactive(snapshot: Snapshot) {
        let votes_count = VoteCount::new();
        let voters = snapshot.to_full_snapshot_info();
        let rewards = calc_voter_rewards(
            votes_count,
            voters,
            Threshold::new(1, HashMap::new(), Vec::new()).unwrap(),
            Rewards::ONE,
        )
        .unwrap();
        assert_eq!(rewards.len(), 0);
    }

    #[proptest]
    fn test_small(snapshot: Snapshot) {
        let voting_keys = snapshot.voting_keys().collect::<Vec<_>>();

        let votes_count = voting_keys
            .iter()
            .enumerate()
            .map(|(i, &key)| {
                (
                    key.to_owned(),
                    if i % 2 == 0 {
                        HashSet::from([Hash::from([0u8; 32])])
                    } else {
                        HashSet::new()
                    },
                )
            })
            .collect::<VoteCount>();
        let n_voters = votes_count
            .iter()
            .filter(|(_, votes)| !votes.is_empty())
            .count();
        let voters = snapshot.to_full_snapshot_info();
        let voters_active = voters
            .clone()
            .into_iter()
            .enumerate()
            .filter(|(i, _utxo)| i % 2 == 0)
            .map(|(_, utxo)| utxo)
            .collect::<Vec<_>>();

        let mut rewards = calc_voter_rewards(
            votes_count.clone(),
            voters,
            Threshold::new(1, HashMap::new(), Vec::new()).unwrap(),
            Rewards::ONE,
        )
        .unwrap();
        let rewards_no_inactive = calc_voter_rewards(
            votes_count,
            voters_active,
            Threshold::new(1, HashMap::new(), Vec::new()).unwrap(),
            Rewards::ONE,
        )
        .unwrap();
        // Rewards should ignore inactive voters
        assert_eq!(rewards, rewards_no_inactive);
        if n_voters > 0 {
            assert_are_close(rewards.values().sum::<Rewards>(), Rewards::ONE);
        } else {
            assert_eq!(rewards.len(), 0);
        }

        let (active, inactive): (Vec<_>, Vec<_>) = voting_keys
            .into_iter()
            .enumerate()
            .partition(|(i, _vk)| i % 2 == 0);

        let active_reward_addresses = active
            .into_iter()
            .flat_map(|(_, vk)| {
                snapshot
                    .contributions_for_voting_key(vk.clone())
                    .into_iter()
                    .map(|c| c.reward_address)
            })
            .collect::<HashSet<_>>();

        assert!(active_reward_addresses
            .iter()
            .all(|addr| rewards.remove(addr).unwrap() > Rewards::ZERO));

        // partial test: does not check that rewards for addresses that delegated to both
        // active and inactive voters only come from active ones
        for (_, vk) in inactive {
            for contrib in snapshot.contributions_for_voting_key(vk.clone()) {
                assert!(rewards.get(&contrib.reward_address).is_none());
            }
        }
    }

    #[test]
    fn test_mapping() {
        let mut raw_snapshot = Vec::new();
        let voting_pub_key = Identifier::from_hex(&hex::encode([0; 32])).unwrap();

        let mut total_stake = 0u64;
        for i in 1..10u64 {
            let stake_public_key = i.to_string();
            let reward_address = i.to_string();
            let delegations = Delegations::New(vec![(voting_pub_key.clone(), 1)]);
            raw_snapshot.push(VotingRegistration {
                stake_public_key,
                voting_power: i.into(),
                reward_address,
                delegations,
                voting_purpose: 0,
            });
            total_stake += i;
        }

        let snapshot = Snapshot::from_raw_snapshot(
            raw_snapshot.into(),
            0.into(),
            Fraction::from(1u64),
            &|_vk: &Identifier| String::new(),
        )
        .unwrap();

        let voters = snapshot.to_full_snapshot_info();

        let rewards = calc_voter_rewards(
            VoteCount::new(),
            voters,
            Threshold::new(0, HashMap::new(), Vec::new()).unwrap(),
            Rewards::ONE,
        )
        .unwrap();
        assert_eq!(rewards.values().sum::<Rewards>(), Rewards::ONE);
        for (addr, reward) in rewards {
            assert_eq!(
                reward,
                addr.parse::<Rewards>().unwrap() / Rewards::from(total_stake)
            );
        }
    }

    #[test]
    fn test_rewards_cap() {
        let mut raw_snapshot = Vec::new();

        for i in 1..10u64 {
            let voting_pub_key = Identifier::from_hex(&hex::encode([i as u8; 32])).unwrap();
            let stake_public_key = i.to_string();
            let reward_address = i.to_string();
            let delegations = Delegations::New(vec![(voting_pub_key.clone(), 1)]);
            raw_snapshot.push(VotingRegistration {
                stake_public_key,
                voting_power: i.into(),
                reward_address,
                delegations,
                voting_purpose: 0,
            });
        }

        let snapshot = Snapshot::from_raw_snapshot(
            raw_snapshot.into(),
            0.into(),
            Fraction::new(1u64, 9u64),
            &|_vk: &Identifier| String::new(),
        )
        .unwrap();

        let voters = snapshot.to_full_snapshot_info();

        let rewards = calc_voter_rewards(
            VoteCount::new(),
            voters,
            Threshold::new(0, HashMap::new(), Vec::new()).unwrap(),
            Rewards::ONE,
        )
        .unwrap();
        assert_are_close(rewards.values().sum::<Rewards>(), Rewards::ONE);
        for (_, reward) in rewards {
            assert_eq!(reward, Rewards::ONE / Rewards::from(9u8));
        }
    }

    #[proptest]
    fn test_per_category_threshold(snapshot: Snapshot) {
        use vit_servicing_station_tests::common::data::ArbitrarySnapshotGenerator;

        let voters = snapshot.to_full_snapshot_info();

        let snapshot = ArbitrarySnapshotGenerator::default().snapshot();
        let mut proposals = snapshot.proposals();
        // for some reasone they are base64 encoded and truncatin is just easier
        for proposal in &mut proposals {
            proposal.proposal.chain_proposal_id.truncate(32);
        }
        let proposals_by_challenge =
            proposals
                .iter()
                .fold(<HashMap<_, Vec<_>>>::new(), |mut acc, prop| {
                    acc.entry(prop.proposal.challenge_id)
                        .or_default()
                        .push(Hash::from(
                            <[u8; 32]>::try_from(prop.proposal.chain_proposal_id.clone()).unwrap(),
                        ));
                    acc
                });
        let per_challenge_threshold = proposals_by_challenge
            .iter()
            .map(|(challenge, p)| (*challenge, p.len()))
            .collect::<HashMap<_, _>>();

        let mut votes_count = voters
            .iter()
            .map(|v| {
                (
                    v.hir.voting_key.clone(),
                    proposals_by_challenge
                        .values()
                        .flat_map(|p| p.iter())
                        .cloned()
                        .collect::<HashSet<_>>(),
                )
            })
            .collect::<Vec<_>>();
        let (_, inactive) = votes_count.split_at_mut(voters.len() / 2);
        for v in inactive {
            v.1.remove(&v.1.iter().next().unwrap().clone());
        }

        let only_active = votes_count
            .clone()
            .into_iter()
            .take(voters.len() / 2)
            .collect::<HashMap<_, _>>();
        let votes_count = votes_count.into_iter().collect::<HashMap<_, _>>();

        let rewards = calc_voter_rewards(
            votes_count,
            voters.clone(),
            Threshold::new(1, per_challenge_threshold.clone(), proposals.clone()).unwrap(),
            Rewards::ONE,
        )
        .unwrap();

        let rewards_only_active = calc_voter_rewards(
            only_active,
            voters,
            Threshold::new(1, per_challenge_threshold, proposals).unwrap(),
            Rewards::ONE,
        )
        .unwrap();

        assert_eq!(rewards_only_active, rewards);
    }
}
