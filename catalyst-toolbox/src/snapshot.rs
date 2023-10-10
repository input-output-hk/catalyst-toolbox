use std::collections::HashMap;

use chain_addr::{Discrimination, Kind};
use jormungandr_lib::crypto::account::Identifier;
use jormungandr_lib::interfaces::{Address, InitialUTxO, Stake, Value};
use serde::{de::Error, Deserialize, Deserializer};
use std::iter::Iterator;

const MAINNET_PAYMENT_PREFIX: &str = "addr";
const TESTNET_PAYMENT_PREFIX: &str = "addr_test";
const MAINNET_STAKE_PREFIX: &str = "stake";
const TESTNET_STAKE_PREFIX: &str = "stake_test";

pub type RewardAddress = String;
pub type StakeAddress = String;

#[derive(Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct CatalystRegistration {
    pub stake_public_key: StakeAddress,
    pub voting_power: Stake,
    #[serde(deserialize_with = "reward_addr_from_hex")]
    pub rewards_address: RewardAddress,
    #[serde(deserialize_with = "identifier_from_hex")]
    pub delegations: Identifier,
}

#[derive(Deserialize, Clone, Debug)]
pub struct RawSnapshot(Vec<CatalystRegistration>);

impl From<Vec<CatalystRegistration>> for RawSnapshot {
    fn from(from: Vec<CatalystRegistration>) -> Self {
        Self(from)
    }
}

#[derive(Clone, Debug)]
pub struct Snapshot {
    // a raw public key is preferred so that we don't have to worry about discrimination when deserializing from
    // a CIP-15 compatible encoding
    inner: HashMap<Identifier, Vec<CatalystRegistration>>,
    stake_threshold: Stake,
}

impl Snapshot {
    pub fn from_raw_snapshot(raw_snapshot: RawSnapshot, stake_threshold: Stake) -> Self {
        Self {
            inner: raw_snapshot
                .0
                .into_iter()
                .filter(|reg| reg.voting_power >= stake_threshold)
                .fold(HashMap::new(), |mut acc, reg| {
                    acc.entry(reg.delegations.clone()).or_default().push(reg);
                    acc
                }),
            stake_threshold,
        }
    }

    pub fn stake_threshold(&self) -> Stake {
        self.stake_threshold
    }

    /// Produces a list of initial UTxOs.
    /// Whether this can be directly converted into an entry in the blockchain
    /// genesis block may depend on further limitations imposed by the blockchain deployment and that
    /// are ignored at this level (e.g. maximum number of outputs in a single fragment)
    pub fn to_block0_initials(
        &self,
        discrimination: Discrimination,
        in_lovelace: bool,
    ) -> Vec<InitialUTxO> {
        self.inner
            .iter()
            .map(|(vk, regs)| {
                let value: Value = regs
                    .iter()
                    .map(|reg| {
                        let value = u64::from(reg.voting_power);
                        if in_lovelace {
                            value / 1_000_000 as u64
                        } else {
                            value
                        }
                    })
                    .sum::<u64>()
                    .into();
                let address: Address =
                    chain_addr::Address(discrimination, Kind::Account(vk.to_inner().into())).into();
                InitialUTxO { address, value }
            })
            .collect::<Vec<_>>()
    }

    pub fn voting_keys(&self) -> impl Iterator<Item = &Identifier> {
        self.inner.keys()
    }

    pub fn registrations_for_voting_key<I: Into<Identifier>>(
        &self,
        voting_public_key: I,
    ) -> Vec<CatalystRegistration> {
        let voting_public_key: Identifier = voting_public_key.into();
        self.inner
            .get(&voting_public_key)
            .cloned()
            .unwrap_or_default()
    }
}

fn identifier_from_hex<'de, D>(deserializer: D) -> Result<Identifier, D::Error>
where
    D: Deserializer<'de>,
{
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum NewDelegationRegistrationInput {
        String(String),
        Int(u64),
    }

    #[derive(Deserialize)]
    #[serde(untagged)]
    enum DelegationRegistration {
        // new registration
        // `"delegations": [["0x221fc1fbcc4abb38c425a922a048af6d492d1260d1b6f055e129385e18f2c603", 1]]`
        New(Vec<Vec<NewDelegationRegistrationInput>>),
        // old registration
        // `"delegations": "0xe8322036dd13fa4576b0e6abe51150c040fc5a9f20a94ecbd918986023354ba3",`
        Old(String),
    }

    let hex = match DelegationRegistration::deserialize(deserializer)? {
        DelegationRegistration::New(delegations) => match delegations
            .get(0)
            .ok_or_else(|| D::Error::custom("Invalid delegations format"))?
            .get(0)
            .ok_or_else(|| D::Error::custom("Invalid delegations format"))?
        {
            NewDelegationRegistrationInput::String(val) => Ok(val.clone()),
            NewDelegationRegistrationInput::Int(_) => {
                Err(D::Error::custom("Invalid delegations format"))
            }
        }?,
        DelegationRegistration::Old(delegations) => delegations,
    };

    Identifier::from_hex(hex.trim_start_matches("0x"))
        .map_err(|e| D::Error::custom(format!("invalid public key {}", e)))
}

fn reward_addr_from_hex<'de, D>(deserializer: D) -> Result<RewardAddress, D::Error>
where
    D: Deserializer<'de>,
{
    enum AddrType {
        Shelley,
        Stake,
    }
    enum NetType {
        Mainnet,
        Testnet,
    }

    // Following cip-0019 specification https://github.com/cardano-foundation/CIPs/blob
    use bech32::ToBase32;
    let bytes = hex::decode(String::deserialize(deserializer)?.trim_start_matches("0x"))
        .map_err(|e| D::Error::custom(format!("invalid hex string: {}", e)))?;

    let addr_prefix = bytes
        .first()
        .ok_or_else(|| D::Error::custom("invalid address format"))?;

    // Shelley addrs: 0x0?, 0x1?, 0x2?, 0x3?, 0x4?, 0x5?, 0x6?, 0x7?
    // Stake addrs: 0xE?, 0xF?
    let addr_type = addr_prefix >> 4 & 0xf;
    // 0 or 1 are valid addrs in the following cases:
    // type = 0x0 -  Testnet network
    // type = 0x1 -  Mainnet network
    let addr_net = addr_prefix & 0xf;

    let addr_type = match addr_type {
        // Shelley
        0x0 | 0x1 | 0x2 | 0x3 | 0x4 | 0x5 | 0x6 | 0x7 => AddrType::Shelley,
        // Stake
        0xf | 0xe => AddrType::Stake,
        _ => {
            return Err(D::Error::custom(format!(
                "invalid address format, incorrect addr type: {}",
                addr_type
            )))
        }
    };

    let addr_net = match addr_net {
        // Mainnet
        0x1 => NetType::Mainnet,
        // Testnet
        0x0 => NetType::Testnet,
        _ => {
            return Err(D::Error::custom(format!(
                "invalid address format, incorrect network tag: {}",
                addr_net
            )))
        }
    };

    let prefix = match (addr_type, addr_net) {
        (AddrType::Shelley, NetType::Mainnet) => MAINNET_PAYMENT_PREFIX,
        (AddrType::Stake, NetType::Mainnet) => MAINNET_STAKE_PREFIX,
        (AddrType::Shelley, NetType::Testnet) => TESTNET_PAYMENT_PREFIX,
        (AddrType::Stake, NetType::Testnet) => TESTNET_STAKE_PREFIX,
    };

    bech32::encode(prefix, bytes.to_base32(), bech32::Variant::Bech32)
        .map_err(|e| D::Error::custom(format!("bech32 encoding failed: {}", e)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use bech32::ToBase32;
    use chain_crypto::{Ed25519, SecretKey};
    use proptest::prelude::*;
    use test_strategy::proptest;

    impl Arbitrary for CatalystRegistration {
        type Parameters = ();
        type Strategy = BoxedStrategy<CatalystRegistration>;

        fn arbitrary_with(_args: Self::Parameters) -> Self::Strategy {
            (any::<([u8; 32], [u8; 32], [u8; 32])>(), 0..45_000_000u64)
                .prop_map(|((stake_key, rewards_addr, voting_key), vp)| {
                    let stake_public_key = hex::encode(stake_key);
                    let reward_address =
                        bech32::encode("stake", &rewards_addr.to_base32(), bech32::Variant::Bech32)
                            .unwrap();
                    let voting_public_key = <SecretKey<Ed25519>>::from_binary(&voting_key)
                        .expect("every binary sequence is a valid secret key")
                        .to_public()
                        .into();
                    let voting_power: Stake = vp.into();
                    CatalystRegistration {
                        stake_public_key,
                        voting_power,
                        rewards_address: reward_address,
                        delegations: voting_public_key,
                    }
                })
                .boxed()
        }
    }

    impl Arbitrary for RawSnapshot {
        type Parameters = ();
        type Strategy = BoxedStrategy<RawSnapshot>;

        fn arbitrary_with(_args: Self::Parameters) -> Self::Strategy {
            any::<Vec<CatalystRegistration>>().prop_map(Self).boxed()
        }
    }

    #[proptest]
    fn test_threshold(raw: RawSnapshot, stake_threshold: u64) {
        let snapshot = Snapshot::from_raw_snapshot(raw, stake_threshold.into());
        assert!(!snapshot
            .inner
            .values()
            .flat_map(|regs| regs.iter().map(|reg| u64::from(reg.voting_power)))
            .any(|voting_power| voting_power < stake_threshold));
    }

    impl Arbitrary for Snapshot {
        type Parameters = ();
        type Strategy = BoxedStrategy<Snapshot>;

        fn arbitrary_with(_args: Self::Parameters) -> Self::Strategy {
            (any::<RawSnapshot>(), 1..u64::MAX)
                .prop_map(|(raw_snapshot, threshold)| {
                    Self::from_raw_snapshot(raw_snapshot, threshold.into())
                })
                .boxed()
        }
    }

    #[test]
    fn test_parsing() {
        let raw: RawSnapshot = serde_json::from_str(
            r#"[
            {
                "delegations": "0xe8322036dd13fa4576b0e6abe51150c040fc5a9f20a94ecbd918986023354ba3",
                "rewards_address": "0xe176cc506ad5d3845e0f51344ca896600df7debf96f58f4af3c1046bd9",
                "stake_public_key": "0x76d416ebfc0a6044b7b00afeeafea330ad1ee2dc3f63b9c4fc5fb685f1dfef01",
                "voting_power": 0,
                "voting_purpose": null,
                "tx_id": 10554899,
                "nonce": 37222821
            },
            {
                "delegations": [
                    [
                      "0x221fc1fbcc4abb38c425a922a048af6d492d1260d1b6f055e129385e18f2c603",
                      1
                    ]
                  ],
                "rewards_address": "0x017514bf116e3625b3d3f534fb0c23b9914876bf82ba6966543688dffb372daf66c2fc9a5d19453a4f429711cba3aee6ae9d78425c23aeaa85",
                "stake_public_key": "0xd96073e70e5c426463e6c5f732712939daa0c6c35313dd989752c7cb672b0b4c",
                "voting_power": 41248637318,
                "voting_purpose": 0,
                "tx_id": 70591630,
                "nonce": 96778096
            },
            {
                "delegations": "0xe8322036dd13fa4576b0e6abe51150c040fc5a9f20a94ecbd918986023354ba3",
                "rewards_address": "0x00cd3be59b212a45b99f2d26bd179c7119e2851c3b7ada415eff504683c7a5c447ebee137a684b65750e8ab5227ffb3199017bdaf069464c11",
                "stake_public_key": "0x76d416ebfc0a6044b7b00afeeafea330ad1ee2dc3f63b9c4fc5fb685f1dfef01",
                "voting_power": 0,
                "voting_purpose": null,
                "tx_id": 10554899,
                "nonce": 37222821
            },
            {
                "delegations": "0xe8322036dd13fa4576b0e6abe51150c040fc5a9f20a94ecbd918986023354ba3",
                "rewards_address": "0xe0b8d7b8e56a3ed89ee21bc062d284d537f843b50b68b905618b130297",
                "stake_public_key": "0x76d416ebfc0a6044b7b00afeeafea330ad1ee2dc3f63b9c4fc5fb685f1dfef01",
                "voting_power": 0,
                "voting_purpose": null,
                "tx_id": 10554899,
                "nonce": 37222821
            }
        ]"#,
        ).unwrap();

        assert_eq!(raw.0.len(), 4);
        assert_eq!(
            raw.0[0],
            CatalystRegistration {
                stake_public_key:
                    "0x76d416ebfc0a6044b7b00afeeafea330ad1ee2dc3f63b9c4fc5fb685f1dfef01".to_string(),
                voting_power: 0.into(),
                delegations: Identifier::from_hex(
                    "e8322036dd13fa4576b0e6abe51150c040fc5a9f20a94ecbd918986023354ba3"
                )
                .unwrap(),
                rewards_address: "stake1u9mvc5r26hfcghs02y6ye2ykvqxl0h4ljm6c7jhncyzxhkg2dz3ks"
                    .to_string(),
            }
        );
        assert_eq!(
            raw.0[1],
            CatalystRegistration {
                stake_public_key:
                    "0xd96073e70e5c426463e6c5f732712939daa0c6c35313dd989752c7cb672b0b4c".to_string(),
                voting_power: 41248637318.into(),
                delegations: Identifier::from_hex(
                    "221fc1fbcc4abb38c425a922a048af6d492d1260d1b6f055e129385e18f2c603"
                )
                .unwrap(),
                rewards_address: "addr1q963f0c3dcmztv7n7560krprhxg5sa4ls2axjej5x6ydl7eh9khkdshunfw3j3f6fapfwywt5whwdt5a0pp9cgaw42zsgxgmxz"
                    .to_string(),
            }
        );
        assert_eq!(
            raw.0[2],
            CatalystRegistration {
                stake_public_key:
                    "0x76d416ebfc0a6044b7b00afeeafea330ad1ee2dc3f63b9c4fc5fb685f1dfef01".to_string(),
                voting_power: 0.into(),
                delegations: Identifier::from_hex(
                    "e8322036dd13fa4576b0e6abe51150c040fc5a9f20a94ecbd918986023354ba3"
                )
                .unwrap(),
                rewards_address: "addr_test1qrxnhevmyy4ytwvl95nt69uuwyv79pgu8dad5s27lagydq785hzy06lwzdaxsjm9w58g4dfz0lanrxgp00d0q62xfsgs5gsfhq"
                    .to_string(),
            }
        );
        assert_eq!(
            raw.0[3],
            CatalystRegistration {
                stake_public_key:
                    "0x76d416ebfc0a6044b7b00afeeafea330ad1ee2dc3f63b9c4fc5fb685f1dfef01".to_string(),
                voting_power: 0.into(),
                delegations: Identifier::from_hex(
                    "e8322036dd13fa4576b0e6abe51150c040fc5a9f20a94ecbd918986023354ba3"
                )
                .unwrap(),
                rewards_address: "stake_test1uzud0w89dgld38hzr0qx955y65mlssa4pd5tjptp3vfs99c4m0ve4"
                    .to_string(),
            }
        );
    }
}
