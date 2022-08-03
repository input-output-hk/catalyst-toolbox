use crate::community_advisors::models::{
    ReviewRanking::{self, *},
    VeteranAdvisorId, VeteranRankingRow,
};
use crate::rewards::Rewards;
use itertools::Itertools;
use rust_decimal::{prelude::ToPrimitive, Decimal};
use std::borrow::Borrow;
use std::collections::{BTreeMap, HashMap};

use serde::Serialize;

#[derive(Serialize)]
pub struct VeteranAdvisorIncentive {
    pub rewards: Rewards,
    pub reputation: u64,
}

pub struct FinalRankingWithConsensus {
    pub review_ranking: ReviewRanking,
    pub consensus: Decimal,
}

pub type VcaRewards = HashMap<VeteranAdvisorId, VeteranAdvisorIncentive>;
pub type EligibilityThresholds = std::ops::RangeInclusive<usize>;

// TODO: for the sake of clarity, introduce a different naming between ca reviews and vca ranking

// Supposing to have a file with all the rankings for each review
// e.g. something like an expanded version of a AdvisorReviewRow
// [proposal_id, advisor, ratings, ..(other fields from AdvisorReviewRow).., ranking (good/excellent/filtered out), vca]

// note that the consensus returned here is either FO/total or ((E+G)/total) because for now we do
// not discriminate between Excellent and Good...
fn calc_final_ranking_with_consensus_per_review(rankings: &[impl Borrow<VeteranRankingRow>]) -> FinalRankingWithConsensus {
    let rankings_majority = Decimal::from(rankings.len()) / Decimal::from(2);
    let ranks = rankings.iter().counts_by(|r| r.borrow().score());

    match (ranks.get(&Excellent), ranks.get(&Good), ranks.get(&FilteredOut)) {
        (_, _, Some(filtered_out)) if Decimal::from(*filtered_out) >= rankings_majority => {
            FinalRankingWithConsensus {
                review_ranking: FilteredOut,
                consensus: Decimal::from(*filtered_out) / Decimal::from(rankings.len()),
            }
        }
        (Some(excellent), maybe_good, _) if Decimal::from(*excellent) > rankings_majority => {
            FinalRankingWithConsensus {
                review_ranking: Excellent,
                consensus: (maybe_good.map(|good| Decimal::from(*good)).unwrap_or_default()
                    + Decimal::from(*excellent)) / Decimal::from(rankings.len()),
            }
        }
        (maybe_excellent, Some(good), _) => {
            FinalRankingWithConsensus {
                review_ranking: Good,
                consensus: (maybe_excellent.map(|excellent| Decimal::from(*excellent)).unwrap_or_default()
                    + Decimal::from(*good)) / Decimal::from(rankings.len()),
            }
        }
        _ => unreachable!(),
    }
}

fn disagreement_modifier<'a>(
    agreement_rate: Decimal,
    modifiers: impl Iterator<Item = &'a (Decimal, Decimal)>,
) -> Decimal {
    for (threshold, modifier) in modifiers {
        if &agreement_rate >= threshold {
            return *modifier;
        }
    }
    // If below lowest threshold, return 0
    Decimal::ZERO
}

fn calc_final_eligible_rankings(
    all_rankings: &HashMap<VeteranAdvisorId, usize>,
    eligible_rankings: HashMap<VeteranAdvisorId, usize>,
    thresholds: EligibilityThresholds,
    modifier_rate: impl Fn(Decimal) -> Decimal,
) -> BTreeMap<VeteranAdvisorId, Rewards> {
    eligible_rankings
        .into_iter()
        .filter_map(|(vca, n_rankings)| {
            if n_rankings < *thresholds.start() {
                return None;
            }

            let to_modifier = modifier_rate(
                Decimal::from(n_rankings) / Decimal::from(*all_rankings.get(&vca).unwrap()),
            );

            let n_rankings = Rewards::from(n_rankings.min(*thresholds.end())) * to_modifier;

            Some((vca, n_rankings))
        })
        .collect()
}


pub fn calculate_veteran_advisors_incentives(
    veteran_rankings: &[VeteranRankingRow],
    total_rewards: Rewards,
    rewards_thresholds: EligibilityThresholds,
    reputation_thresholds: EligibilityThresholds,
    rewards_mod_args: Vec<(Decimal, Decimal)>,
    reputation_mod_args: Vec<(Decimal, Decimal)>,
    minimum_consensus: Decimal,
) -> HashMap<VeteranAdvisorId, VeteranAdvisorIncentive> {
    let final_rankings_with_consensus_per_review = veteran_rankings
        .iter()
        .into_group_map_by(|ranking| ranking.review_id())
        .into_iter()
        .map(|(review, rankings)| (review, calc_final_ranking_with_consensus_per_review(&rankings)))
        .collect::<BTreeMap<_, _>>();

    let rankings_per_vca = veteran_rankings
        .iter()
        .counts_by(|ranking| ranking.vca.clone());

    let eligible_rankings_per_vca = veteran_rankings
        .iter()
        .filter(|ranking| {
            let final_ranking_with_consensus = final_rankings_with_consensus_per_review
                .get(&ranking.review_id())
                .unwrap();

            final_ranking_with_consensus.review_ranking
                .is_positive() == ranking.score().is_positive()
                || final_ranking_with_consensus.consensus < minimum_consensus
        })
        .counts_by(|ranking| ranking.vca.clone());

    let reputation_eligible_rankings = calc_final_eligible_rankings(
        &rankings_per_vca,
        eligible_rankings_per_vca.clone(),
        reputation_thresholds,
        |agreement| disagreement_modifier(agreement, reputation_mod_args.iter()),
    );

    let rewards_eligible_rankings = calc_final_eligible_rankings(
        &rankings_per_vca,
        eligible_rankings_per_vca,
        rewards_thresholds,
        |agreement| disagreement_modifier(agreement, rewards_mod_args.iter()),
    );

    let tot_rewards_eligible_rankings = rewards_eligible_rankings.values().sum::<Rewards>();

    reputation_eligible_rankings
        .into_iter()
        .zip(rewards_eligible_rankings.into_iter())
        .map(|((vca, reputation), (_vca2, reward))| {
            assert_eq!(vca, _vca2); // the use of BTreeMaps ensures iteration is consistent
            (
                vca,
                VeteranAdvisorIncentive {
                    reputation: reputation.to_u64().expect("result does not fit into u64"),
                    rewards: total_rewards * reward / tot_rewards_eligible_rankings,
                },
            )
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::{distributions::Alphanumeric, Rng};
    use rust_decimal_macros::dec;
    use std::iter::Iterator;

    const THRESHOLDS: [Decimal; 3] = [dec!(0.9), dec!(0.8), dec!(0.7)];
    // A [1.25, 1, 0.75] modifer is equivalent to a [1, 0.8, 0.6] slashing, but keep the
    // former to adhere precisely to gov specifications.
    const REWARDS_DISAGREEMENT_MODIFIERS: [Decimal; 3] = [dec!(1.25), Decimal::ONE, dec!(0.75)];
    const REPUTATION_DISAGREEMENT_MODIFIERS: [Decimal; 3] =
        [Decimal::ONE, Decimal::ONE, Decimal::ONE];

    const VCA_1: &str = "vca1";
    const VCA_2: &str = "vca2";
    const VCA_3: &str = "vca3";
    const SIMPLE_MINIMUM_CONSENSUS: Decimal = dec!(.5);
    const QUALIFIED_MINIMUM_CONSENSUS: Decimal = dec!(.7);

    struct RandomIterator;
    impl Iterator for RandomIterator {
        type Item = String;
        fn next(&mut self) -> Option<Self::Item> {
            Some(
                (0..10)
                    .map(|_| rand::thread_rng().sample(Alphanumeric) as char)
                    .collect(),
            )
        }
    }

    fn gen_dummy_rankings(
        assessor: String,
        n_excellent: u32,
        n_good: u32,
        n_filtered_out: u32,
        vca: impl Iterator<Item = String>,
    ) -> Vec<VeteranRankingRow> {
        (0..n_excellent)
            .map(|_| ReviewRanking::Excellent)
            .chain((0..n_good).map(|_| ReviewRanking::Good))
            .chain((0..n_filtered_out).map(|_| ReviewRanking::FilteredOut))
            .zip(vca)
            .map(|(ranking, vca)| VeteranRankingRow::dummy(ranking, assessor.clone(), vca))
            .collect()
    }

    #[test]
    fn final_ranking_is_correct() {
        assert!(matches!(
            calc_final_ranking_with_consensus_per_review(&gen_dummy_rankings("".into(), 5, 5, 5, RandomIterator)),
            FinalRankingWithConsensus {
                review_ranking: Good,
                consensus
            } if consensus == (dec!(10) / dec!(15))
        ));

        assert!(matches!(
            calc_final_ranking_with_consensus_per_review(&gen_dummy_rankings("".into(), 4, 2, 5, RandomIterator)),
            FinalRankingWithConsensus {
                review_ranking: Good,
                consensus
            } if consensus == (dec!(6) / dec!(11))

        ));

        assert!(matches!(
            calc_final_ranking_with_consensus_per_review(&gen_dummy_rankings("".into(), 4, 1, 5, RandomIterator)),
            FinalRankingWithConsensus {
                review_ranking: FilteredOut,
                consensus,
            } if consensus == (dec!(5) / dec!(10))
        ));

        assert!(matches!(
            calc_final_ranking_with_consensus_per_review(&gen_dummy_rankings("".into(), 3, 1, 1, RandomIterator)),
            FinalRankingWithConsensus {
                review_ranking: Excellent,
                consensus,
            } if consensus == (dec!(4) / dec!(5))
        ));
    }

    #[test]
    fn lower_threshold() {
        let vcas = vec![VCA_1.to_owned(), VCA_2.to_owned()].into_iter();
        let vca2_only = vec![VCA_2.to_owned()].into_iter();
        let total_rewards = Rewards::ONE;
        let rankings = gen_dummy_rankings("1".into(), 1, 1, 0, vcas)
            .into_iter()
            .chain(gen_dummy_rankings("2".into(), 1, 0, 0, vca2_only))
            .collect::<Vec<_>>();
        // only vca with more than 2 reviews get reputation and rewards
        let results = calculate_veteran_advisors_incentives(
            &rankings,
            total_rewards,
            2..=2,
            2..=2,
            THRESHOLDS
                .into_iter()
                .zip(REWARDS_DISAGREEMENT_MODIFIERS.into_iter())
                .collect(),
            THRESHOLDS
                .into_iter()
                .zip(REPUTATION_DISAGREEMENT_MODIFIERS.into_iter())
                .collect(),
            SIMPLE_MINIMUM_CONSENSUS,
        );
        assert!(results.get(VCA_1).is_none());
        let res = results.get(VCA_2).unwrap();
        assert_eq!(res.reputation, 2);
        assert_eq!(res.rewards, total_rewards);
    }

    #[test]
    fn upper_threshold() {
        let vcas = vec![VCA_1.to_owned(), VCA_2.to_owned()].into_iter();
        let vca2_only = vec![VCA_2.to_owned()].into_iter();
        let total_rewards = Rewards::ONE;
        let rankings = gen_dummy_rankings("1".into(), 1, 1, 0, vcas)
            .into_iter()
            .chain(gen_dummy_rankings("2".into(), 1, 0, 0, vca2_only))
            .collect::<Vec<_>>();
        let results = calculate_veteran_advisors_incentives(
            &rankings,
            total_rewards,
            1..=1,
            1..=1,
            THRESHOLDS
                .into_iter()
                .zip(REWARDS_DISAGREEMENT_MODIFIERS.into_iter())
                .collect(),
            THRESHOLDS
                .into_iter()
                .zip(REPUTATION_DISAGREEMENT_MODIFIERS.into_iter())
                .collect(),
            SIMPLE_MINIMUM_CONSENSUS,
        );
        let res1 = results.get(VCA_1).unwrap();
        assert_eq!(res1.reputation, 1);
        assert_eq!(res1.rewards, Rewards::ONE / Rewards::from(2));
        let res2 = results.get(VCA_2).unwrap();
        assert_eq!(res2.reputation, 1);
        assert_eq!(res2.rewards, Rewards::ONE / Rewards::from(2));
    }

    fn are_close(a: Decimal, b: Decimal) -> bool {
        const DECIMAL_PRECISION: u32 = 10;
        a.round_dp(DECIMAL_PRECISION) == b.round_dp(DECIMAL_PRECISION)
    }

    #[test]
    fn disagreement_modifier_rate() {
        let total_rewards = Rewards::ONE;
        let inputs = [
            (Rewards::new(6, 1), Rewards::ZERO, Rewards::ZERO),
            (Rewards::new(7, 1), Rewards::new(75, 2), Rewards::ONE),
            (Rewards::new(8, 1), Rewards::ONE, Rewards::ONE),
            (Rewards::new(9, 1), Rewards::new(125, 2), Rewards::ONE),
        ];
        for (vca3_agreement, reward_modifier, reputation_modifier) in inputs {
            let rankings = (0..100)
                .flat_map(|i| {
                    let vcas =
                        vec![VCA_1.to_owned(), VCA_2.to_owned(), VCA_3.to_owned()].into_iter();
                    let (good, filtered_out) = if Rewards::from(i) < vca3_agreement * Rewards::from(100)
                    {
                        (3, 0)
                    } else {
                        (2, 1)
                    };
                    gen_dummy_rankings(i.to_string(), 0, good, filtered_out, vcas).into_iter()
                })
                .collect::<Vec<_>>();
            let results_simple_consensus = calculate_veteran_advisors_incentives(
                &rankings,
                total_rewards,
                1..=200,
                1..=200,
                THRESHOLDS
                    .into_iter()
                    .zip(REWARDS_DISAGREEMENT_MODIFIERS.into_iter())
                    .collect(),
                THRESHOLDS
                    .into_iter()
                    .zip(REPUTATION_DISAGREEMENT_MODIFIERS.into_iter())
                    .collect(),
                SIMPLE_MINIMUM_CONSENSUS,
            );
            let vca3_expected_reward_portion_simple_consensus = vca3_agreement * Rewards::from(100) * reward_modifier;
            dbg!(vca3_expected_reward_portion_simple_consensus);
            dbg!(vca3_agreement, reward_modifier, reputation_modifier);
            let vca3_expected_rewards_simple_consensus = total_rewards
                / (Rewards::from(125 * 2) + vca3_expected_reward_portion_simple_consensus)
                * vca3_expected_reward_portion_simple_consensus;
            let res_vca3_simple_consensus = results_simple_consensus.get(VCA_3).unwrap();
            assert_eq!(
                res_vca3_simple_consensus.reputation,
                (Rewards::from(100) * vca3_agreement * reputation_modifier)
                    .to_u64()
                    .unwrap()
            );
            assert!(are_close(res_vca3_simple_consensus.rewards, vca3_expected_rewards_simple_consensus));


            let results_qualified_consensus = calculate_veteran_advisors_incentives(
                &rankings,
                total_rewards,
                1..=200,
                1..=200,
                THRESHOLDS
                    .into_iter()
                    .zip(REWARDS_DISAGREEMENT_MODIFIERS.into_iter())
                    .collect(),
                THRESHOLDS
                    .into_iter()
                    .zip(REPUTATION_DISAGREEMENT_MODIFIERS.into_iter())
                    .collect(),
                QUALIFIED_MINIMUM_CONSENSUS,
            );

            let vca3_expected_reward_portion_qualified_consensus = Rewards::from(100) * dec!(1.25); // low consensus so max reward modifier, agreement ratio doesn't count as all and rankings are all eligible
            dbg!(vca3_expected_reward_portion_qualified_consensus);
            dbg!(vca3_agreement, reward_modifier, reputation_modifier);

            let vca3_expected_rewards_qualified_consensus = total_rewards
                / (Rewards::from(125 * 2) + vca3_expected_reward_portion_qualified_consensus)
                * vca3_expected_reward_portion_qualified_consensus; // 1/3 of the reward

            let res_vca3_qualified_consensus = results_qualified_consensus.get(VCA_3).unwrap();


            assert_eq!(
                res_vca3_qualified_consensus.reputation,
                (Rewards::from(100)) // all assessment are valid since consensus is low (2/3 < 0.7)
                    .to_u64()
                    .unwrap()
            );
            assert!(are_close(res_vca3_qualified_consensus.rewards, vca3_expected_rewards_qualified_consensus));
        }
    }
}
