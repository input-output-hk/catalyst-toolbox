mod de;

use serde::{Deserialize, Deserializer};

pub use de::{AdvisorReviewRow, ReviewScore};

pub enum ProposalStatus {
    Funded,
    NotFunded,
}

#[derive(Deserialize)]
pub struct ApprovedProposalRow {
    #[serde(rename(deserialize = "internal_id"))]
    pub proposal_id: String,
    pub status: ProposalStatus,
    pub requested_dollars: String,
}

impl<'de> Deserialize<'de> for ProposalStatus {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let status: String = String::deserialize(deserializer)?;
        Ok(match status.to_lowercase().as_ref() {
            "funded" => ProposalStatus::Funded,
            _ => ProposalStatus::NotFunded,
        })
    }
}
