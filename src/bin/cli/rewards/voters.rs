use super::Error;
use catalyst_toolbox::rewards::voters::{
    calculate_rewards, vote_count_with_addresses, AddressesVoteCount, Record, VoteCount,
};
use jcli_lib::jcli_lib::block::Common;
use jormungandr_lib::interfaces::Block0Configuration;
use std::path::PathBuf;
use structopt::StructOpt;

#[derive(StructOpt)]
#[structopt(rename_all = "kebab-case")]
pub struct VotersRewards {
    #[structopt(flatten)]
    pub(crate) common: Common,
    /// Reward (in LOVELACE) to be distributed
    #[structopt(long)]
    pub(crate) total_rewards: u64,

    #[structopt(long)]
    pub(crate) votes_count_path: PathBuf,

    #[structopt(long, default_value)]
    pub(crate) vote_threshold: u64,
}

fn write_rewards_results(common: Common, records: Vec<Record>) -> Result<(), Error> {
    let writer = common.open_output()?;
    let mut csv_writer = csv::Writer::from_writer(writer);
    for record in records {
        csv_writer.serialize(&record).map_err(Error::Csv)?;
    }
    Ok(())
}

impl VotersRewards {
    pub fn exec(self) -> Result<(), Error> {
        let VotersRewards {
            common,
            total_rewards,
            votes_count_path,
            vote_threshold,
        } = self;
        let block = common.input.load_block()?;
        let block0 = Block0Configuration::from_block(&block)
            .map_err(jcli_lib::jcli_lib::block::Error::BuildingGenesisFromBlock0Failed)?;

        let vote_count: VoteCount = serde_json::from_reader(jcli_lib::utils::io::open_file_read(
            &Some(votes_count_path),
        )?)?;

        let addresses_vote_count: AddressesVoteCount =
            vote_count_with_addresses(vote_count, &block0);

        let records =
            calculate_rewards(addresses_vote_count, &block0, vote_threshold, total_rewards)?;

        write_rewards_results(common, records)?;
        Ok(())
    }
}
