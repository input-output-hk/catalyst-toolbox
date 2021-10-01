use super::tags::TagsMap;
use serde::Deserialize;
use std::ops::Range;
use vit_servicing_station_lib::db::models::community_advisors_reviews::ReviewTag;

#[derive(Deserialize)]
pub struct ValidAssessments {
    proposal_id: String,
    #[serde(alias = "Proposal URL")]
    idea_url: String,
    #[serde(alias = "Assessor")]
    assessor: String,
    #[serde(alias = "Impact / Alignment Note")]
    impact_alignment_note: String,
    #[serde(alias = "Impact / Alignment Rating")]
    impact_alignment_rating: u8,
    #[serde(alias = "Feasibility Note")]
    feasibility_note: String,
    #[serde(alias = "Feasibility Rating")]
    feasibility_rating: u8,
    #[serde(alias = "Auditability Note")]
    auditability_note: String,
    #[serde(alias = "Auditability Rating")]
    auditability_rating: u8,
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    #[test]
    fn test_deserialize() {
        let file_path = PathBuf::from("../../../resources/testing/valid_assessments.csv");
    }
}
