mod archive;
mod live;
mod snapshot;
mod voters;

use archive::ArchiveCommand;
use live::LiveStatsCommand;
use snapshot::SnapshotCommand;
use structopt::StructOpt;
use voters::InitialVotersCommand;

#[derive(StructOpt, Debug)]
pub enum Stats {
    Voters(InitialVotersCommand),
    Live(LiveStatsCommand),
    Archive(ArchiveCommand),
    Snapshot(SnapshotCommand),
}

impl Stats {
    pub fn exec(self) -> Result<(), catalyst_toolbox::stats::Error> {
        match self {
            Self::Voters(voters) => voters.exec(),
            Self::Live(live) => live.exec(),
            Self::Archive(archive) => archive.exec(),
            Self::Snapshot(snapshot) => snapshot.exec(),
        }
    }
}