use fixed::types::U64F64;

use chain_addr::{Discrimination, Kind};
use chain_impl_mockchain::transaction::UnspecifiedAccountIdentifier;
use rust_decimal::Decimal;
use std::collections::{HashMap, HashSet};

use jormungandr_lib::interfaces::{Address, Block0Configuration, Initial};

pub const ADA_TO_LOVELACE_FACTOR: u64 = 1_000_000;
pub type Rewards = Decimal;

pub fn calculate_active_stake<'address>(
    committee_keys: &HashSet<Address>,
    block0: &'address Block0Configuration,
    active_addresses: &ActiveAddresses,
) -> (u64, HashMap<&'address Address, u64>) {
    let mut total_stake: u64 = 0;
    let mut stake_per_voter: HashMap<&'address Address, u64> = HashMap::new();

    for fund in &block0.initial {
        match fund {
            Initial::Fund(fund) => {
                for utxo in fund {
                    // Exclude committee addresses (managed by IOG) and
                    // non active voters from total active stake for the purpose of calculating
                    // voter rewards
                    if !committee_keys.contains(&utxo.address)
                        && active_addresses.contains(&utxo.address)
                    {
                        let value: u64 = utxo.value.into();
                        total_stake += value;
                        let entry = stake_per_voter.entry(&utxo.address).or_default();
                        *entry += value;
                    }
                }
            }
            Initial::Cert(_) => {}
            Initial::LegacyFund(_) => {}
        }
    }
    (total_stake, stake_per_voter)
}

pub fn calculate_reward_share<'address>(
    total_stake: u64,
    stake_per_voter: &HashMap<&'address Address, u64>,
    active_addresses: &ActiveAddresses,
) -> HashMap<&'address Address, Rewards> {
    stake_per_voter
        .iter()
        .map(|(k, v)| {
            // if it doesnt appear in the votes count, it means it did not vote
            let reward = if active_addresses.contains(k) {
                Rewards::from(*v) / Rewards::from(total_stake)
            } else {
                Rewards::ZERO
            };
            (*k, reward)
        })
        .collect()
}

/// get the proportional reward from a share and total rewards amount
pub fn reward_from_share(share: Rewards, total_reward: u64) -> Rewards {
    Rewards::from(total_reward) * share
}

pub type VoteCount = HashMap<String, u64>;
pub type ActiveAddresses = HashSet<Address>;

pub fn active_addresses(
    vote_count: VoteCount,
    block0: &Block0Configuration,
    threshold: u64,
) -> ActiveAddresses {
    let discrimination = block0.blockchain_configuration.discrimination;
    vote_count
        .into_iter()
        .filter_map(|(account_hex, count)| {
            if count >= threshold {
                Some(
                    account_hex_to_address(account_hex, discrimination)
                        .expect("Valid hex encoded UnspecifiedAccountIdentifier"),
                )
            } else {
                None
            }
        })
        .collect()
}

fn account_hex_to_address(
    account_hex: String,
    discrimination: Discrimination,
) -> Result<Address, hex::FromHexError> {
    let mut buffer = [0u8; 32];
    hex::decode_to_slice(account_hex, &mut buffer)?;
    let identifier: UnspecifiedAccountIdentifier = UnspecifiedAccountIdentifier::from(buffer);
    Ok(Address::from(chain_addr::Address(
        discrimination,
        Kind::Account(
            identifier
                .to_single_account()
                .expect("Only single accounts are supported")
                .into(),
        ),
    )))
}
