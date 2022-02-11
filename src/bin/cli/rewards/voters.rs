use super::Error;
use catalyst_toolbox::rewards::voters::{
    active_addresses, calculate_active_stake, calculate_reward_share, reward_from_share,
    ActiveAddresses, Rewards, VoteCount, ADA_TO_LOVELACE_FACTOR,
};

use chain_addr::{Discrimination, Kind};
use chain_impl_mockchain::vote::CommitteeId;
use jcli_lib::jcli_lib::block::Common;
use jormungandr_lib::interfaces::{Address, Block0Configuration};

use structopt::StructOpt;

use std::collections::{HashMap, HashSet};
use std::ops::Div;
use std::path::PathBuf;

#[derive(StructOpt)]
#[structopt(rename_all = "kebab-case")]
pub struct VotersRewards {
    #[structopt(flatten)]
    common: Common,
    /// Reward (in LOVELACE) to be distributed
    #[structopt(long)]
    total_rewards: u64,

    /// Stake threshold to be able to participate in a Catalyst sidechain
    /// Registrations with less than the threshold associated to the stake address
    /// will be ignored
    #[structopt(long)]
    registration_threshold: u64,

    #[structopt(long)]
    votes_count_path: PathBuf,

    /// Number of votes required to be able to receive voter rewards
    #[structopt(long, default_value)]
    vote_threshold: u64,
}

fn write_rewards_results(
    common: Common,
    stake_per_voter: &HashMap<&Address, u64>,
    share_results: &HashMap<&Address, Rewards>,
    total_rewards: u64,
) -> Result<(), Error> {
    let writer = common.open_output()?;
    let header = [
        "Address",
        "Stake of the voter (ADA)",
        "Reward for the voter (ADA)",
        "Reward for the voter (lovelace)",
    ];
    let mut csv_writer = csv::Writer::from_writer(writer);
    csv_writer.write_record(&header).map_err(Error::Csv)?;

    for (address, share) in share_results.iter() {
        let stake = stake_per_voter.get(*address).unwrap();
        let voter_reward = reward_from_share(*share, total_rewards);
        let record = [
            address.to_string(),
            stake.to_string(),
            voter_reward / Rewards::from(ADA_TO_LOVELACE_FACTOR).to_string(),
            voter_reward.trunc().to_string(),
        ];
        csv_writer.write_record(&record).map_err(Error::Csv)?;
    }
    Ok(())
}

impl VotersRewards {
    pub fn exec(self) -> Result<(), Error> {
        let VotersRewards {
            common,
            total_rewards,
            registration_threshold,
            votes_count_path,
            vote_threshold,
        } = self;
        let block = common.input.load_block()?;
        let block0 = Block0Configuration::from_block(&block)
            .map_err(jcli_lib::jcli_lib::block::Error::BuildingGenesisFromBlock0Failed)?;

        let vote_count: VoteCount = serde_json::from_reader(jcli_lib::utils::io::open_file_read(
            &Some(votes_count_path),
        )?)?;

        let active_addresses = active_addresses(vote_count, &block0, vote_threshold);

        let committee_keys: HashSet<Address> = block0
            .blockchain_configuration
            .committees
            .iter()
            .cloned()
            .map(|id| {
                let id = CommitteeId::from(id);
                let pk = id.public_key();

                chain_addr::Address(Discrimination::Production, Kind::Account(pk)).into()
            })
            .collect();

        let (total_active_stake, stake_per_voter) =
            calculate_active_stake(&committee_keys, &block0, &active_addresses);
        let rewards =
            calculate_reward_share(total_active_stake, &stake_per_voter, &active_addresses);
        write_rewards_results(common, &stake_per_voter, &rewards, total_rewards)?;
        Ok(())
    }
}
