use super::Error;
use catalyst_toolbox::rewards::voters::{calc_voter_rewards, Rewards, VoteCount};
use catalyst_toolbox::snapshot::{registration::MainnetRewardAddress, SnapshotInfo};
use catalyst_toolbox::utils::assert_are_close;
use voting_hir::VotingGroup;

use jcli_lib::jcli_lib::block::Common;

use structopt::StructOpt;

use std::collections::BTreeMap;
use std::path::PathBuf;

#[derive(StructOpt)]
#[structopt(rename_all = "kebab-case")]
pub struct VotersRewards {
    #[structopt(flatten)]
    common: Common,
    /// Reward (in LOVELACE) to be distributed
    #[structopt(long)]
    total_rewards: u64,

    /// Path to a json encoded list of `SnapshotInfo`
    #[structopt(long)]
    snapshot_info_path: PathBuf,

    #[structopt(long)]
    votes_count_path: PathBuf,

    /// Number of votes required to be able to receive voter rewards
    #[structopt(long, default_value)]
    vote_threshold: u64,

    direct_voter_group: VotingGroup,

    representative_group: VotingGroup,
}

fn write_rewards_results(
    common: Common,
    rewards: &BTreeMap<MainnetRewardAddress, Rewards>,
) -> Result<(), Error> {
    let writer = common.open_output()?;
    let header = ["Address", "Reward for the voter (lovelace)"];
    let mut csv_writer = csv::Writer::from_writer(writer);
    csv_writer.write_record(&header).map_err(Error::Csv)?;

    for (address, rewards) in rewards.iter() {
        let record = [address.to_string(), rewards.trunc().to_string()];
        csv_writer.write_record(&record).map_err(Error::Csv)?;
    }
    Ok(())
}

impl VotersRewards {
    pub fn exec(self) -> Result<(), Error> {
        let VotersRewards {
            common,
            total_rewards,
            snapshot_info_path,
            votes_count_path,
            vote_threshold,
            direct_voter_group,
            representative_group,
        } = self;

        let vote_count: VoteCount = serde_json::from_reader(jcli_lib::utils::io::open_file_read(
            &Some(votes_count_path),
        )?)?;

        let snapshot: Vec<SnapshotInfo> = serde_json::from_reader(
            jcli_lib::utils::io::open_file_read(&Some(snapshot_info_path))?,
        )?;

        let results = calc_voter_rewards(
            vote_count,
            vote_threshold,
            snapshot,
            Rewards::from(total_rewards),
            representative_group,
            direct_voter_group,
        )?;

        let actual_rewards = results.values().sum::<Rewards>();
        assert_are_close(actual_rewards, Rewards::from(total_rewards));

        write_rewards_results(common, &results)?;
        Ok(())
    }
}
