use catalyst_toolbox::ideascale::{
    build_challenges, build_fund, build_proposals, fetch_all, CustomFieldTags,
    Error as IdeascaleError, Scores, Sponsors,
};
use jcli_lib::utils::io as io_utils;
use jormungandr_lib::interfaces::VotePrivacy;
use std::collections::HashSet;

use structopt::StructOpt;

use serde::de::DeserializeOwned;
use serde::Serialize;
use std::path::{Path, PathBuf};

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error(transparent)]
    Ideascale(#[from] IdeascaleError),

    #[error(transparent)]
    Io(#[from] std::io::Error),

    #[error(transparent)]
    Csv(#[from] csv::Error),

    #[error(transparent)]
    Serde(#[from] serde_json::Error),
}

#[derive(Debug, StructOpt)]
pub enum Ideascale {
    Import(Import),
}

// We need this type because structopt uses Vec<String> as a special type, so it is not compatible
// with custom parsers feature.
type Filters = Vec<String>;

#[derive(Debug, StructOpt)]
#[structopt(rename_all = "kebab")]
pub struct Import {
    /// Fund number id
    #[structopt(long)]
    fund: usize,

    /// Fund goal explanation
    #[structopt(long)]
    fund_goal: String,

    /// Stage label: stage identifiers that links to assessments scores in ideascale
    #[structopt(long, default_value = "Assess")]
    stage_label: String,

    /// ideascale API token
    #[structopt(long, env = "IDEASCALE_API_TOKEN")]
    api_token: String,

    /// Fund approval threshold setting
    #[structopt(long)]
    threshold: i64,

    /// either "public" or "private"
    #[structopt(long)]
    chain_vote_type: VotePrivacy,

    /// Path to folder where fund, challenges and proposals json files will be dumped
    #[structopt(long)]
    output_dir: PathBuf,

    /// Path to proposal scores csv file
    #[structopt(long)]
    scores: Option<PathBuf>,

    /// Path to proposal scores csv file
    #[structopt(long)]
    sponsors: Option<PathBuf>,

    /// Path to json or json like file containing tag configuration for ideascale custom fields
    #[structopt(long)]
    tags: Option<PathBuf>,

    /// Path to json or json like file containing list of excluded proposal ids
    #[structopt(long)]
    excluded_proposals: Option<PathBuf>,

    /// Ideascale stages list,
    #[structopt(long, parse(from_str=parse_from_csv), default_value = "Governance phase;Assess QA")]
    stages_filters: Filters,
}

impl Ideascale {
    pub fn exec(&self) -> Result<(), Error> {
        match self {
            Ideascale::Import(import) => import.exec(),
        }
    }
}

impl Import {
    fn exec(&self) -> Result<(), Error> {
        let Import {
            fund,
            fund_goal,
            stage_label,
            api_token,
            threshold,
            chain_vote_type,
            output_dir: save_folder,
            scores,
            sponsors,
            tags,
            excluded_proposals,
            stages_filters,
        } = self;

        let tags: CustomFieldTags = if let Some(tags_path) = tags {
            read_json_from_file(tags_path)?
        } else {
            Default::default()
        };

        let excluded_proposals: HashSet<u32> = if let Some(excluded_path) = excluded_proposals {
            read_json_from_file(excluded_path)?
        } else {
            Default::default()
        };

        let runtime = tokio::runtime::Builder::new_multi_thread()
            .enable_io()
            .enable_time()
            .build()?;

        let scores = read_scores_file(scores)?;
        let sponsors = read_sponsors_file(sponsors)?;
        let idescale_data = runtime.block_on(fetch_all(
            *fund,
            &stage_label.to_lowercase(),
            &stages_filters.iter().map(AsRef::as_ref).collect::<Vec<_>>(),
            &excluded_proposals,
            api_token.clone(),
        ))?;

        let funds = build_fund(*fund as i32, fund_goal.clone(), *threshold);
        let challenges = build_challenges(*fund as i32, &idescale_data, sponsors);
        let proposals = build_proposals(
            &idescale_data,
            &challenges,
            &scores,
            &chain_vote_type.to_string(),
            *fund,
            &tags,
        );

        let mut challenges: Vec<_> = challenges.values().collect();
        // even if final id type is string, they are just sequentially added, so it should be safe
        // to parse and unwrap here
        challenges.sort_by_key(|c| c.id.parse::<i32>().unwrap());

        dump_content_to_file(
            funds,
            save_folder
                .join(format!("fund{}_funds.json", fund))
                .as_path(),
        )?;

        dump_content_to_file(
            challenges,
            save_folder
                .join(format!("fund{}_challenges.json", fund))
                .as_path(),
        )?;

        dump_content_to_file(
            proposals,
            save_folder
                .join(format!("fund{}_proposals.json", fund))
                .as_path(),
        )?;

        Ok(())
    }
}

fn dump_content_to_file(content: impl Serialize, file_path: &Path) -> Result<(), Error> {
    let writer = jcli_lib::utils::io::open_file_write(&Some(file_path))?;
    serde_json::to_writer_pretty(writer, &content).map_err(Error::Serde)
}

fn read_json_from_file<T: DeserializeOwned>(file_path: &Path) -> Result<T, Error> {
    let reader = io_utils::open_file_read(&Some(file_path))?;
    serde_json::from_reader(reader).map_err(Error::Serde)
}

fn parse_from_csv(s: &str) -> Filters {
    s.split(';').map(|x| x.to_string()).collect()
}

fn read_scores_file(path: &Option<PathBuf>) -> Result<Scores, Error> {
    let mut scores = Scores::new();
    if let Some(path) = path {
        let mut reader = csv::Reader::from_path(path)?;
        for record in reader.records() {
            let record = record?;
            let proposal_id: u32 = record
                .get(0)
                .expect("Proposal ids should be present in scores file second column")
                .parse()
                .expect("Proposal ids should be integers");
            let rating_given: f32 = record
                .get(1)
                .expect("Ratings should be present in scores file third column")
                .parse()
                .expect("Ratings should be floats [0, 5]");
            scores.insert(proposal_id, rating_given);
        }
    }
    Ok(scores)
}

fn read_sponsors_file(path: &Option<PathBuf>) -> Result<Sponsors, Error> {
    let mut sponsors = Sponsors::new();

    if let Some(path) = path {
        let mut reader = csv::Reader::from_path(path)?;
        for record in reader.records() {
            let record = record?;
            let challenge_url: String = record
                .get(0)
                .expect("Challenge url should be present in scores file first column")
                .to_string();
            let sponsor_name: String = record
                .get(1)
                .expect("Sponsor name should be present in scores file second column")
                .to_string();
            sponsors.insert(challenge_url, sponsor_name);
        }
    }
    Ok(sponsors)
}
