use serde::Deserialize;

use vit_servicing_station_lib::db::models::community_advisors_reviews::ReviewTag;

#[derive(Deserialize)]
pub struct AdvisorReviewRow {
    pub proposal_id: String,
    #[serde(alias = "Proposal URL")]
    pub idea_url: String,
    #[serde(alias = "Assessor")]
    pub assessor: String,
    #[serde(alias = "Impact / Alignment Note")]
    impact_alignment_note: String,
    #[serde(alias = "Impact / Alignment Rating")]
    pub impact_alignment_rating: u8,
    #[serde(alias = "Feasibility Note")]
    feasibility_note: String,
    #[serde(alias = "Feasibility Rating")]
    pub feasibility_rating: u8,
    #[serde(alias = "Auditability Note")]
    auditability_note: String,
    #[serde(alias = "Auditability Rating")]
    pub auditability_rating: u8,
    pub excellent: u32,
    pub good: u32,
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    #[test]
    fn test_deserialize() {
        let file_path = PathBuf::from("../../../resources/testing/valid_assessments.csv");
    }
}
