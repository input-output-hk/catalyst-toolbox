use crate::utils::serde::deserialize_truthy_falsy;
use serde::Deserialize;

// TODO: When using this for the reviews import cmd remove allow dead_code
#[allow(dead_code)]
#[derive(Deserialize)]
pub struct AdvisorReviewRow {
    pub proposal_id: String,
    #[serde(alias = "Idea URL")]
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
    #[serde(alias = "Excellent", deserialize_with = "deserialize_truthy_falsy")]
    excellent: bool,
    #[serde(alias = "Good", deserialize_with = "deserialize_truthy_falsy")]
    good: bool,
}

pub enum ReviewScore {
    Excellent,
    Good,
}

impl AdvisorReviewRow {
    pub fn score(&self) -> ReviewScore {
        match (self.excellent, self.good) {
            (true, false) => ReviewScore::Excellent,
            (false, true) => ReviewScore::Excellent,
            _ => {
                // This should never happen
                panic!(
                    "Invalid combination of scores from assessor {} for proposal {}",
                    self.assessor, self.proposal_id
                )
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::community_advisors::models::AdvisorReviewRow;
    use crate::utils::csv as csv_utils;
    use std::path::PathBuf;

    #[test]
    fn test_deserialize() {
        let file_path = PathBuf::from("./resources/testing/valid_assessments.csv");
        let data: Vec<AdvisorReviewRow> = csv_utils::load_data_from_csv(&file_path).unwrap();
        assert_eq!(data.len(), 1);
    }
}
