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
        let reviews: Vec<veterans::VeteranRankingRow> = csv::load_data_from_csv<_, b','>(&from)?;
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

#[cfg(test)]
mod tests {
    use crate::cli::rewards::veterans::VeteransRewards;
    use catalyst_toolbox::rewards::veterans::VeteranAdvisorReward;
    use jcli_lib::utils::io;
    use rust_decimal::prelude::FromStr;
    use std::io::BufRead;

    #[test]
    fn test_output_csv() {
        let resource_input = "./resources/testing/veteran_reviews_count.csv";
        let tmp_file = assert_fs::NamedTempFile::new("outfile.csv").unwrap();

        let export = VeteransRewards {
            from: resource_input.into(),
            to: tmp_file.path().into(),
            total_rewards: 1000.into(),
        };

        export.exec().unwrap();
        let reader = io::open_file_read(&Some(tmp_file.path())).unwrap();
        let expected_reward = [50u32, 100, 200, 300, 350].map(VeteranAdvisorReward::from);
        for (line, expected) in reader.lines().zip(expected_reward.iter()) {
            let line = line.unwrap();
            let res: Vec<&str> = line.split(',').collect();
            let reward = VeteranAdvisorReward::from_str(res[1]).unwrap();
            assert_eq!(&reward, expected);
        }
    }
}
