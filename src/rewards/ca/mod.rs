mod excel;
mod funding;
mod lottery;
mod models;

use serde::Deserialize;

pub type Ca = String;
pub type ProposalId = String;

#[derive(Deserialize)]
pub enum ReviewGrade {
    Excellent,
    Good,
    FilteredOut,
}
