use catalyst_toolbox::snapshot::{voting_group::RepsVotersAssigner, RawSnapshot, Snapshot};
use jcli_lib::utils::{
    output_file::{Error as OutputFileError, OutputFile},
    output_format::{Error as OutputFormatError, OutputFormat},
};
use jormungandr_lib::interfaces::Value;
use rust_decimal::Decimal;
use std::fs::File;
use std::io::Write;
use std::path::PathBuf;
use structopt::StructOpt;
use thiserror::Error;

const DEFAULT_DIRECT_VOTER_GROUP: &str = "voter";
const DEFAULT_REPRESENTATIVE_GROUP: &str = "rep";

/// Process raw registrations into blockchain initials
#[derive(StructOpt)]
#[structopt(rename_all = "kebab-case")]
pub struct SnapshotCmd {
    /// Path to the file containing all CIP-15 compatible registrations in json format.
    #[structopt(short, long, parse(from_os_str))]
    snapshot: PathBuf,
    /// Registrations voting power threshold for eligibility
    #[structopt(short, long)]
    min_stake_threshold: Value,
    /// Maximum stake in percent that could be controlled by a single entity
    /// in the resulting HIR
    max_stake_percent: Option<Decimal>,

    /// Voter group to assign direct voters to.
    /// If empty, defaults to "voter"
    #[structopt(short, long)]
    direct_voters_group: Option<String>,

    /// Voter group to assign representatives to.
    /// If empty, defaults to "rep"
    #[structopt(short, long)]
    representatives_group: Option<String>,

    /// Url of the representative db api server
    #[structopt(short, long)]
    reps_db_api_url: String,

    #[structopt(flatten)]
    output: OutputFile,

    #[structopt(flatten)]
    output_format: OutputFormat,
}

#[derive(Debug, Error)]
pub enum Error {
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
    #[error(transparent)]
    OutputFile(#[from] OutputFileError),
    #[error(transparent)]
    OutputFormat(#[from] OutputFormatError),
    #[error(transparent)]
    Reps(#[from] catalyst_toolbox::snapshot::voting_group::Error),
}

impl SnapshotCmd {
    pub fn exec(self) -> Result<(), Error> {
        let raw_snapshot: RawSnapshot = serde_json::from_reader(File::open(&self.snapshot)?)?;
        let direct_voter = self
            .direct_voters_group
            .unwrap_or_else(|| DEFAULT_DIRECT_VOTER_GROUP.into());
        let representative = self
            .representatives_group
            .unwrap_or_else(|| DEFAULT_REPRESENTATIVE_GROUP.into());
        let assigner = RepsVotersAssigner::new(direct_voter, representative, self.reps_db_api_url)?;
        let initials =
            Snapshot::from_raw_snapshot(raw_snapshot, self.threshold).to_voter_hir(&assigner);
        let mut out_writer = self.output.open()?;
        let content = self
            .output_format
            .format_json(serde_json::to_value(initials)?)?;
        out_writer.write_all(content.as_bytes())?;
        Ok(())
    }
}
