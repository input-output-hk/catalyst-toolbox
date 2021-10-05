mod funding;
mod lottery;

use crate::community_advisors::models::{AdvisorReviewRow, ReviewScore};
use crate::rewards::ca::funding::{Funds, ProposalRewardSlots};
use std::collections::HashMap;

pub use funding::FundSetting;

pub type Ca = String;
pub type ProposalId = String;

type ProposalsRewards = HashMap<ProposalId, ProposalReward>;
pub type CaRewards = HashMap<Ca, Funds>;
pub type ProposalsReviews = HashMap<ProposalId, Vec<AdvisorReviewRow>>;

enum ProposalRewardsState {
    // Proposal has the exact quantity reviews to be rewarded
    Exact,
    // Proposal has less reviews as needed so some of the funds should go back into the rewards pool
    Unfilled(Funds),
    // It has more reviews than fitted rewards
    OverLoaded,
}

struct ProposalReward {
    pub state: ProposalRewardsState,
    pub funds: Funds,
}

fn proposal_rewards_state(
    proposal_reviews: &[AdvisorReviewRow],
    proposal_fund: Funds,
    rewards_slots: &ProposalRewardSlots,
) -> ProposalRewardsState {
    let filled_slots: u64 = proposal_reviews
        .iter()
        .map(|review| match review.score() {
            ReviewScore::Excellent => rewards_slots.excellent_slots,
            ReviewScore::Good => rewards_slots.good_slots,
        })
        .sum();

    if filled_slots < rewards_slots.filled_slots {
        let unfilled_funds =
            proposal_fund * (Funds::from(filled_slots) / Funds::from(rewards_slots.filled_slots));
        ProposalRewardsState::Unfilled(unfilled_funds)
    } else if filled_slots > rewards_slots.filled_slots {
        ProposalRewardsState::OverLoaded
    } else {
        ProposalRewardsState::Exact
    }
}

fn calculate_funds_per_proposal(
    funding: &FundSetting,
    proposal_reviews: &ProposalsReviews,
    rewards_slots: &ProposalRewardSlots,
) -> ProposalsRewards {
    let per_proposal_reward = funding.funds_per_proposal(proposal_reviews.len() as u64);

    // check rewards and split extra until there is no more to split
    let proposal_rewards_states: HashMap<ProposalId, ProposalRewardsState> = proposal_reviews
        .iter()
        .map(|(id, reviews)| {
            (
                id.clone(),
                proposal_rewards_state(reviews, per_proposal_reward, rewards_slots),
            )
        })
        .collect();

    let underbudget_funds: Funds = proposal_rewards_states
        .values()
        .map(|state| match state {
            ProposalRewardsState::Unfilled(value) => value.clone(),
            _ => Funds::from(0u64),
        })
        .sum();

    let underbudget_rewards = underbudget_funds / Funds::from(proposal_reviews.len() as u64);

    proposal_rewards_states
        .into_iter()
        .map(|(id, state)| {
            let funds = match state {
                ProposalRewardsState::Unfilled(unfilled_funds) => {
                    per_proposal_reward - unfilled_funds
                }
                _ => per_proposal_reward + underbudget_rewards,
            };
            (id, ProposalReward { state, funds })
        })
        .collect()
}
