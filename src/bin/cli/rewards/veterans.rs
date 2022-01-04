use super::Error;
use catalyst_toolbox::rewards::veterans;
use catalyst_toolbox::rewards::Rewards;
use catalyst_toolbox::utils::csv;
use std::path::PathBuf;
use structopt::StructOpt;

#[derive(StructOpt)]
#[structopt(rename_all = "kebab-case")]
pub struct VeteransRewards {
    /// Reviews csv file path
    from: PathBuf,

    /// Results file output path
    to: PathBuf,

    /// Reward to be distributed
    #[structopt(long = "total-rewards")]
    total_rewards: Rewards,

    /// Minimum number of rankings for each vca to be considered for reputatin and rewards
    /// distribution
    #[structopt(long, short)]
    minimum_rankings: usize,

    /// Cutoff for monetary rewards: ranking more reviews than this limit will not result in more rewards
    #[structopt(long, short)]
    maximum_ranking_for_rewards: usize,

    /// Cutoff for reputation: ranking more reviews than this limit will not result in more reputation awarded
    #[structopt(long, short)]
    maximum_ranking_for_reputation: usize,
}

impl VeteransRewards {
    pub fn exec(self) -> Result<(), Error> {
        let Self {
            from,
            to,
            total_rewards,
            minimum_rankings,
            maximum_ranking_for_reputation,
            maximum_ranking_for_rewards,
        } = self;
        let reviews: Vec<veterans::VeteranRankingRow> = csv::load_data_from_csv::<_, b','>(&from)?;
        let results = veterans::calculate_veteran_advisors_incentives(
            &reviews,
            total_rewards,
            minimum_rankings..=maximum_ranking_for_rewards,
            minimum_rankings..=maximum_ranking_for_reputation,
        );
        csv::dump_data_to_csv(&results.into_iter().collect::<Vec<_>>(), &to)?;

        Ok(())
    }
}
