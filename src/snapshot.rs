use std::collections::HashMap;

use chain_addr::{Discrimination, Kind};
use jormungandr_lib::crypto::account::Identifier;
use jormungandr_lib::interfaces::{Address, Initial, InitialUTxO, Stake, Value};
use serde::Deserialize;

pub type MainnetRewardAddress = String;
pub type MainnetStakeAddress = String;

#[derive(Deserialize, Clone, Debug)]
pub struct CatalystRegistration {
    pub stake: Stake,
    pub reward_addr: MainnetRewardAddress,
    pub voting_pub_key: Identifier,
}

#[derive(Deserialize, Clone)]
pub struct RawSnapshot(HashMap<MainnetStakeAddress, CatalystRegistration>);

#[derive(Clone, Debug)]
pub struct Snapshot {
    // a raw public key is preferred so that we don't have to worry about discrimination when deserializing from
    // a CIP-15 compatible encoding
    inner: HashMap<Identifier, Vec<(MainnetRewardAddress, Stake)>>,
    stake_threshold: Stake,
}

impl Snapshot {
    pub fn from_raw_snapshot(raw_snapshot: RawSnapshot, stake_threshold: Stake) -> Self {
        Self {
            inner: raw_snapshot
                .0
                .into_iter()
                .filter(|(_stake_key, reg)| reg.stake > stake_threshold)
                .fold(
                    HashMap::new(),
                    |mut acc,
                     (
                        _stake,
                        CatalystRegistration {
                            stake,
                            reward_addr,
                            voting_pub_key,
                        },
                    )| {
                        acc.entry(voting_pub_key)
                            .or_default()
                            .push((reward_addr, stake));
                        acc
                    },
                ),
            stake_threshold,
        }
    }

    pub fn stake_threshold(&self) -> Stake {
        self.stake_threshold
    }

    pub fn to_block0_initials(&self, discrimination: Discrimination) -> Initial {
        Initial::Fund(
            self.inner
                .iter()
                .map(|(vk, regs)| {
                    let value: Value = regs
                        .iter()
                        .map(|(_, stake)| u64::from(*stake))
                        .sum::<u64>()
                        .into();
                    let address: Address =
                        chain_addr::Address(discrimination, Kind::Account(vk.to_inner().into()))
                            .into();
                    InitialUTxO { address, value }
                })
                .collect::<Vec<_>>(),
        )
    }

    pub fn voting_keys(&self) -> Vec<Identifier> {
        self.inner.keys().cloned().collect()
    }

    pub fn registrations_for_voting_key<I: Into<Identifier>>(
        &self,
        voting_pub_key: I,
    ) -> Vec<CatalystRegistration> {
        let voting_pub_key: Identifier = voting_pub_key.into();
        self.inner
            .get(&voting_pub_key)
            .cloned()
            .unwrap_or_default()
            .into_iter()
            .map(|(reward_addr, stake)| CatalystRegistration {
                reward_addr,
                stake,
                voting_pub_key: voting_pub_key.clone(),
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chain_crypto::{Ed25519, SecretKey};
    use quickcheck::{Arbitrary, Gen};

    impl Arbitrary for RawSnapshot {
        fn arbitrary<G: Gen>(g: &mut G) -> Self {
            let n_registrations = usize::arbitrary(g);

            let mut raw_snapshot = HashMap::new();

            for i in 0..n_registrations {
                let stake_key = String::arbitrary(g);
                let reward_addr = String::arbitrary(g);
                let voting_pub_key = <SecretKey<Ed25519>>::arbitrary(g).to_public().into();
                let stake: Stake = u64::arbitrary(g).into();
                raw_snapshot.insert(
                    stake_key,
                    CatalystRegistration {
                        stake,
                        reward_addr,
                        voting_pub_key,
                    },
                );
            }

            Self(raw_snapshot)
        }
    }

    impl Arbitrary for Snapshot {
        fn arbitrary<G: Gen>(g: &mut G) -> Self {
            Self::from_raw_snapshot(<_>::arbitrary(g), (u64::arbitrary(g)).into())
        }
    }

    impl From<HashMap<MainnetStakeAddress, CatalystRegistration>> for RawSnapshot {
        fn from(from: HashMap<MainnetStakeAddress, CatalystRegistration>) -> Self {
            Self(from)
        }
    }
}
