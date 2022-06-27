use jormungandr_lib::crypto::account::Identifier;
use jormungandr_lib::interfaces::Value;
use serde::{de::Error, Deserialize};

pub type MainnetRewardAddress = String;
pub type MainnetStakeAddress = String;

/// The voting registration/delegation format as introduced in CIP-36,
/// which is a generalizatin of CIP-15, allowing to distribute
/// voting power among multiple keys in a single transaction and
/// to tag the purpose of the vote.
#[derive(Deserialize, Clone, Debug, PartialEq)]
pub struct VotingRegistration {
    pub stake_public_key: MainnetStakeAddress,
    pub voting_power: Value,
    /// Shelley address discriminated for the same network this transaction is submitted to.
    #[serde(
        deserialize_with = "deser::reward_addr_from_hex",
        serialize_with = "ser::reward_addr_to_hex"
    )]
    pub reward_address: MainnetRewardAddress,
    pub delegations: Delegations,
    /// 0 = Catalyst, assumed 0 for old legacy registrations
    #[serde(default)]
    pub voting_purpose: u64,
}

/// To allow backward compatibility and avoid requiring existing users to
/// re-register we still consider valid old CIP-15 registrations, with the
/// simple correspondence between the two described in CIP-36.
#[derive(Clone, Debug, PartialEq)]
pub enum Delegations {
    /// Tuples of (voting key, weight)
    New(Vec<(Identifier, u32)>),
    Legacy(Identifier),
}

mod deser {
    use super::*;
    use chain_crypto::{Ed25519, PublicKey};
    use serde::de::{self, Deserialize, Deserializer, SeqAccess, Visitor};
    use std::fmt;

    pub(super) struct IdentifierDef(pub(super) Identifier);
    struct VotingKeyVisitor;

    impl<'de> Visitor<'de> for VotingKeyVisitor {
        type Value = Identifier;

        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            formatter.write_str("a voting key as described in CIP-36")
        }

        fn visit_str<E: de::Error>(self, v: &str) -> Result<Self::Value, E> {
            Identifier::from_hex(v.trim_start_matches("0x"))
                .map_err(|e| E::custom(format!("invalid voting key {}: {}", v, e)))
        }

        fn visit_bytes<E: de::Error>(self, v: &[u8]) -> Result<Self::Value, E> {
            <PublicKey<Ed25519>>::from_binary(v)
                .map_err(|e| E::custom(format!("invalid voting key: {}", e)))
                .map(Self::Value::from)
        }
    }

    impl<'de> Deserialize<'de> for IdentifierDef {
        fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where
            D: Deserializer<'de>,
        {
            if deserializer.is_human_readable() {
                deserializer.deserialize_str(VotingKeyVisitor).map(Self)
            } else {
                deserializer.deserialize_bytes(VotingKeyVisitor).map(Self)
            }
        }
    }

    impl<'de> Deserialize<'de> for Delegations {
        fn deserialize<D>(deserializer: D) -> Result<Delegations, D::Error>
        where
            D: Deserializer<'de>,
        {
            struct DelegationsVisitor;

            impl<'de> Visitor<'de> for DelegationsVisitor {
                type Value = Delegations;

                fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                    formatter.write_str("delegations as described in CIP-36")
                }

                // If we have to visit a str that means we are trying to deserialize the legacy
                // variant of the enum with a single voting key in hex format
                fn visit_str<E: de::Error>(self, v: &str) -> Result<Self::Value, E> {
                    Ok(Self::Value::Legacy(VotingKeyVisitor.visit_str(v)?))
                }

                // Same thing for bytes
                fn visit_bytes<E: de::Error>(self, v: &[u8]) -> Result<Self::Value, E> {
                    Ok(Self::Value::Legacy(VotingKeyVisitor.visit_bytes(v)?))
                }

                // If we are visiting a sequence of values that means we are trying to deserialize
                // the new variant of the enum
                fn visit_seq<V>(self, mut seq: V) -> Result<Self::Value, V::Error>
                where
                    V: SeqAccess<'de>,
                {
                    let mut vks = Vec::with_capacity(seq.size_hint().unwrap_or(1));
                    while let Some((vk, weight)) = seq.next_element::<(IdentifierDef, u32)>()? {
                        vks.push((vk.0, weight));
                    }
                    if vks.is_empty() {
                        return Err(V::Error::custom("expected at least one delegation"));
                    }

                    Ok(Self::Value::New(vks))
                }
            }
            // This is to support untagged variants (i.e. both legacy and new delegations format) without
            // any overhead since knowing the data type alone is enough to discriminate the variants.
            //
            // A safer way to do this would be to try parsing each variant instead of relying on the
            // deserializer to know the data type, which is not available in some formats like bincode,
            deserializer.deserialize_any(DelegationsVisitor)
        }
    }

    pub fn reward_addr_from_hex<'de, D>(deserializer: D) -> Result<MainnetRewardAddress, D::Error>
    where
        D: Deserializer<'de>,
    {
        use bech32::ToBase32;
        let bytes = hex::decode(String::deserialize(deserializer)?.trim_start_matches("0x"))
            .map_err(|e| D::Error::custom(format!("invalid hex string: {}", e)))?;
        bech32::encode("stake", &bytes.to_base32(), bech32::Variant::Bech32)
            .map_err(|e| D::Error::custom(format!("bech32 encoding failed: {}", e)))
    }
}

mod ser {
    use bech32::FromBase32;
    use serde::ser::{Error, Serializer};

    #[allow(dead_code)]
    pub fn reward_addr_to_hex<S>(obj: &str, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let (_, data, _) = bech32::decode(obj)
            .map_err(|e| S::Error::custom(format!("bech32 decoding failed: {}", e)))?;
        let data = Vec::<u8>::from_base32(&data)
            .map_err(|e| S::Error::custom(format!("invalid hex string: {}", e)))?;
        serializer.serialize_str(&format!("0x{}", hex::encode(data)))
    }
}

#[cfg(any(feature = "test-api", test))]
mod test_api {
    use super::deser::IdentifierDef;
    use super::*;
    use serde::{ser::SerializeStruct, Serialize, Serializer};

    impl Serialize for VotingRegistration {
        fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: Serializer,
        {
            let mut state = serializer.serialize_struct("VotingRegistration", 5)?;
            state.serialize_field("stake_public_key", &self.stake_public_key)?;
            state.serialize_field("voting_power", &self.voting_power)?;
            state.serialize_field("delegations", &self.delegations)?;
            state.serialize_field("reward_address", &self.reward_address)?;
            state.serialize_field("voting_purpose", &self.voting_purpose)?;
            state.end()
        }
    }

    // This is only to make it easier to test the Deserialize impl
    impl Serialize for IdentifierDef {
        fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: Serializer,
        {
            if serializer.is_human_readable() {
                serializer.serialize_str(&format!("0x{}", self.0.to_hex()))
            } else {
                serializer.serialize_bytes(self.0.as_ref().as_ref())
            }
        }
    }

    // This is only to make it easier to test the Deserialize impl
    impl Serialize for Delegations {
        fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: Serializer,
        {
            match self {
                Self::Legacy(key) => IdentifierDef(key.clone()).serialize(serializer),
                Self::New(vec) => vec
                    .iter()
                    .map(|(vk, weight)| (IdentifierDef(vk.clone()), weight))
                    .collect::<Vec<_>>()
                    .serialize(serializer),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bech32::ToBase32;
    use chain_crypto::{Ed25519, SecretKey};
    use proptest::collection::vec;
    use proptest::prelude::*;
    use serde::{Deserialize, Serialize};
    use serde_test::{assert_de_tokens, Configure, Token};
    use test_strategy::proptest;

    #[test]
    pub fn serialize_bijection() {
        #[derive(Deserialize, Serialize, Clone, Debug, PartialEq)]
        struct Test {
            #[serde(
                deserialize_with = "deser::reward_addr_from_hex",
                serialize_with = "ser::reward_addr_to_hex"
            )]
            pub reward_address: MainnetRewardAddress,
        }

        let content =
            "{\"reward_address\":\"0xe1ffff2912572257b59dca84c965e4638a09f1524af7a15787eb0d8a46\"}";
        let test: Test = serde_json::from_str(content).unwrap();
        assert_eq!(content, serde_json::to_string(&test).unwrap());
    }

    impl Arbitrary for Delegations {
        type Parameters = ();
        type Strategy = BoxedStrategy<Delegations>;

        fn arbitrary_with(_args: Self::Parameters) -> Self::Strategy {
            vec(any::<([u8; 32], u32)>(), 1..10) // Default is 0..100
                .prop_map(|vec| {
                    Delegations::New(
                        vec.into_iter()
                            .map(|(key, weight)| {
                                (
                                    <SecretKey<Ed25519>>::from_binary(&key)
                                        .unwrap()
                                        .to_public()
                                        .into(),
                                    weight,
                                )
                            })
                            .collect(),
                    )
                })
                .boxed()
                .prop_union(
                    any::<[u8; 32]>()
                        .prop_map(|key| {
                            Delegations::Legacy(
                                <SecretKey<Ed25519>>::from_binary(&key)
                                    .unwrap()
                                    .to_public()
                                    .into(),
                            )
                        })
                        .boxed(),
                )
                .boxed()
        }
    }

    impl Arbitrary for VotingRegistration {
        type Parameters = ();
        type Strategy = BoxedStrategy<VotingRegistration>;

        fn arbitrary_with(_args: Self::Parameters) -> Self::Strategy {
            (any::<([u8; 32], [u8; 32], Delegations)>(), 0..45_000_000u64)
                .prop_map(|((stake_key, rewards_addr, delegations), vp)| {
                    let stake_public_key = hex::encode(stake_key);
                    let reward_address =
                        bech32::encode("stake", &rewards_addr.to_base32(), bech32::Variant::Bech32)
                            .unwrap();
                    let voting_power: Value = vp.into();
                    VotingRegistration {
                        stake_public_key,
                        voting_power,
                        reward_address,
                        delegations,
                        voting_purpose: 0,
                    }
                })
                .boxed()
        }
    }

    #[test]
    fn parse_example() {
        assert_de_tokens(
            &Delegations::New(vec![
                (
                    Identifier::from_hex(
                        "a6a3c0447aeb9cc54cf6422ba32b294e5e1c3ef6d782f2acff4a70694c4d1663",
                    )
                    .unwrap(),
                    3,
                ),
                (
                    Identifier::from_hex(
                        "00588e8e1d18cba576a4d35758069fe94e53f638b6faf7c07b8abd2bc5c5cdee",
                    )
                    .unwrap(),
                    1,
                ),
            ])
            .readable(),
            &[
                Token::Seq { len: None },
                Token::Seq { len: None },
                Token::Str("0xa6a3c0447aeb9cc54cf6422ba32b294e5e1c3ef6d782f2acff4a70694c4d1663"),
                Token::U32(3),
                Token::SeqEnd,
                Token::Seq { len: None },
                Token::Str("0x00588e8e1d18cba576a4d35758069fe94e53f638b6faf7c07b8abd2bc5c5cdee"),
                Token::U32(1),
                Token::SeqEnd,
                Token::SeqEnd,
            ],
        );
    }

    #[proptest]
    fn serde_json(d: Delegations) {
        assert_eq!(
            serde_json::from_str::<Delegations>(&serde_json::to_string(&d).unwrap()).unwrap(),
            d
        )
    }

    #[proptest]
    fn serde_yaml(d: Delegations) {
        assert_eq!(
            serde_yaml::from_str::<Delegations>(&serde_yaml::to_string(&d).unwrap()).unwrap(),
            d
        )
    }

    #[test]
    fn test_empty_delegations_are_rejected() {
        assert!(serde_json::from_str::<Delegations>(r#"[]"#,).is_err());
    }

    #[test]
    fn test_u64_weight_is_rejected() {
        assert!(serde_json::from_str::<Delegations>(r#"[["0xa6a3c0447aeb9cc54cf6422ba32b294e5e1c3ef6d782f2acff4a70694c4d1663", 4294967296]]"#,).is_err());
    }

    #[test]
    fn test_legacy_delegation_is_ok() {
        assert!(serde_json::from_str::<Delegations>(
            r#""0xa6a3c0447aeb9cc54cf6422ba32b294e5e1c3ef6d782f2acff4a70694c4d1663""#,
        )
        .is_ok());
    }

    #[test]
    fn test_u32_weight_is_ok() {
        serde_json::from_str::<Delegations>(
            r#"[["0xa6a3c0447aeb9cc54cf6422ba32b294e5e1c3ef6d782f2acff4a70694c4d1663", 4294967295]]"#,
        )
        .unwrap();
    }
}
