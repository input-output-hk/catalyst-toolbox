use crate::utils::serde::deserialize_truthy_falsy;
use serde::Deserialize;

/// (Proposal Id, Assessor Id), an assessor cannot assess the same proposal more than once
pub type AdvisorReviewId = (String, String);
pub type VeteranAdvisorId = String;

#[derive(Deserialize)]
pub struct AdvisorReviewRow {
    pub proposal_id: String,
    #[serde(alias = "Idea URL")]
    pub idea_url: String,
    #[serde(alias = "Assessor")]
    pub assessor: String,
    #[serde(alias = "Impact / Alignment Note")]
    pub impact_alignment_note: String,
    #[serde(alias = "Impact / Alignment Rating")]
    pub impact_alignment_rating: u8,
    #[serde(alias = "Feasibility Note")]
    pub feasibility_note: String,
    #[serde(alias = "Feasibility Rating")]
    pub feasibility_rating: u8,
    #[serde(alias = "Auditability Note")]
    pub auditability_note: String,
    #[serde(alias = "Auditability Rating")]
    pub auditability_rating: u8,
    #[serde(alias = "Excellent", deserialize_with = "deserialize_truthy_falsy")]
    excellent: bool,
    #[serde(alias = "Good", deserialize_with = "deserialize_truthy_falsy")]
    good: bool,
    #[serde(
        default,
        alias = "Filtered Out",
        deserialize_with = "deserialize_truthy_falsy"
    )]
    filtered_out: bool,
}

#[derive(Deserialize)]
pub struct VeteranRankingRow {
    pub proposal_id: String,
    #[serde(alias = "Assessor")]
    pub assessor: String,
    #[serde(alias = "Excellent", deserialize_with = "deserialize_truthy_falsy")]
    excellent: bool,
    #[serde(alias = "Good", deserialize_with = "deserialize_truthy_falsy")]
    good: bool,
    #[serde(
        default,
        alias = "Filtered Out",
        deserialize_with = "deserialize_truthy_falsy"
    )]
    filtered_out: bool,
    pub vca: VeteranAdvisorId,
}

#[derive(Hash, Clone, PartialEq, Eq, Debug)]
pub enum ReviewRanking {
    Excellent,
    Good,
    FilteredOut,
    NA, // not reviewed by vCAs
}

impl ReviewRanking {
    pub fn is_positive(&self) -> bool {
        matches!(self, Self::Excellent | Self::Good)
    }
}

impl VeteranRankingRow {
    pub fn score(&self) -> ReviewRanking {
        ranking_from_bools(self.excellent, self.good, self.filtered_out)
    }

    pub fn review_id(&self) -> AdvisorReviewId {
        (self.proposal_id.clone(), self.assessor.clone())
    }
}

impl AdvisorReviewRow {
    pub fn score(&self) -> ReviewRanking {
        ranking_from_bools(self.excellent, self.good, self.filtered_out)
    }
}

fn ranking_from_bools(excellent: bool, good: bool, filtered_out: bool) -> ReviewRanking {
    match (excellent, good, filtered_out) {
        (true, false, false) => ReviewRanking::Excellent,
        (false, true, false) => ReviewRanking::Good,
        (false, false, true) => ReviewRanking::FilteredOut,
        (false, false, false) => ReviewRanking::NA,
        _ => {
            // This should never happen, from the source of information a review could be either
            // Excellent or Good or not assessed. It cannot be both and it is considered
            // a malformed information input.
            panic!(
                "Invalid combination of scores {} {} {}",
                excellent, good, filtered_out
            )
        }
    }
}

#[cfg(test)]
mod tests {
    use super::ReviewRanking;
    use crate::community_advisors::models::AdvisorReviewRow;
    use crate::utils::csv as csv_utils;
    use rand::{distributions::Alphanumeric, Rng};
    use std::path::PathBuf;

    #[test]
    fn test_deserialize() {
        let file_path = PathBuf::from("./resources/testing/valid_assessments.csv");
        let data: Vec<AdvisorReviewRow> =
            csv_utils::load_data_from_csv::<_, b','>(&file_path).unwrap();
        assert_eq!(data.len(), 1);
    }

    impl AdvisorReviewRow {
        pub fn dummy(score: ReviewRanking) -> Self {
            Self::with_assessor(
                (0..10)
                    .map(|_| rand::thread_rng().sample(Alphanumeric) as char)
                    .collect(),
                score,
            )
        }

        pub fn with_assessor(assessor: String, score: ReviewRanking) -> Self {
            let (excellent, good, filtered_out) = match score {
                ReviewRanking::Good => (false, true, false),
                ReviewRanking::Excellent => (true, false, false),
                ReviewRanking::FilteredOut => (false, false, true),
                ReviewRanking::NA => (false, false, false),
            };

            AdvisorReviewRow {
                proposal_id: String::new(),
                idea_url: String::new(),
                assessor,
                impact_alignment_note: String::new(),
                impact_alignment_rating: 0,
                feasibility_note: String::new(),
                feasibility_rating: 0,
                auditability_note: String::new(),
                auditability_rating: 0,
                excellent,
                good,
                filtered_out,
            }
        }
    }
}
