use serde::Deserialize;

#[derive(Deserialize, Eq, PartialEq)]
struct AggregatedReview {}

#[derive(Deserialize, Eq, PartialEq)]
struct EligibleReview {}

#[derive(Deserialize, Eq, PartialEq)]
struct ExcludedReview {}

#[derive(Deserialize, Eq, PartialEq)]
struct Assessor {
    pub excluded: bool,
}
