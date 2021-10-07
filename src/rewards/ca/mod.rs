mod funding;
mod lottery;

use crate::community_advisors::models::{AdvisorReviewRow, ReviewScore};
use lottery::TicketsDistribution;

use std::collections::{HashMap, HashSet};

pub use crate::rewards::ca::funding::ProposalRewardSlots;
pub use funding::{FundSetting, Funds};

pub type Ca = String;
pub type ProposalId = String;
// Lets match to the same type as the funds, but naming it funds would be confusing
pub type Rewards = Funds;

type ProposalsFunds = HashMap<ProposalId, ProposalReward>;
pub type CaRewards = HashMap<Ca, Rewards>;
pub type ProposalsReviews = HashMap<ProposalId, Vec<AdvisorReviewRow>>;
pub type ApprovedProposals = HashSet<ProposalId>;

enum ProposalFundsState {
    // Proposal has the exact quantity reviews to be rewarded
    Exact,
    // Proposal has less reviews as needed so some of the funds should go back into the rewards pool
    Unfilled(Funds),
    // It has more reviews than fitted rewards
    OverLoaded,
}

struct ProposalReward {
    pub state: ProposalFundsState,
    pub funds: Funds,
}

fn proposal_rewards_state(
    proposal_reviews: &[AdvisorReviewRow],
    proposal_fund: Funds,
    rewards_slots: &ProposalRewardSlots,
) -> ProposalFundsState {
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
        ProposalFundsState::Unfilled(unfilled_funds)
    } else if filled_slots > rewards_slots.filled_slots {
        ProposalFundsState::OverLoaded
    } else {
        ProposalFundsState::Exact
    }
}

fn calculate_funds_per_proposal(
    proposal_reviews: &ProposalsReviews,
    approved_proposals: &ApprovedProposals,
    funding: &FundSetting,
    rewards_slots: &ProposalRewardSlots,
) -> ProposalsFunds {
    let per_proposal_reward = funding.funds_per_proposal(proposal_reviews.len() as u64);
    let bonus_proposals_rewards = funding.bonus_funds_per_proposal(approved_proposals.len() as u64);

    // check rewards and split extra until there is no more to split
    let proposal_rewards_states: HashMap<ProposalId, ProposalFundsState> = proposal_reviews
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
            ProposalFundsState::Unfilled(value) => value.clone(),
            _ => Funds::from(0u64),
        })
        .sum();

    let underbudget_rewards = underbudget_funds / Funds::from(proposal_reviews.len() as u64);

    proposal_rewards_states
        .into_iter()
        .map(|(id, state)| {
            let bonus_funds = approved_proposals
                .contains(&id)
                .then(|| bonus_proposals_rewards)
                .unwrap_or(Funds::from(0u64));
            let funds = match state {
                ProposalFundsState::Unfilled(unfilled_funds) => {
                    per_proposal_reward - unfilled_funds
                }
                _ => per_proposal_reward + underbudget_rewards,
            };
            (
                id,
                ProposalReward {
                    state,
                    funds: funds + bonus_funds,
                },
            )
        })
        .collect()
}

fn load_tickets_from_reviews(
    proposal_reviews: &[AdvisorReviewRow],
    rewards_slots: &ProposalRewardSlots,
) -> TicketsDistribution {
    let mut tickets_distribution = TicketsDistribution::new();
    for review in proposal_reviews {
        let entry = tickets_distribution
            .entry(review.assessor.clone())
            .or_insert(0);
        let tickets_to_add = match review.score() {
            ReviewScore::Excellent => rewards_slots.excellent_slots,
            ReviewScore::Good => rewards_slots.good_slots,
        };
        *entry += tickets_to_add;
    }
    tickets_distribution
}

fn distribute_rewards(
    funds: Funds,
    cas: &TicketsDistribution,
    rewards_slots: &ProposalRewardSlots,
) -> CaRewards {
    let rewards_per_ticket = funds / Funds::from(rewards_slots.filled_slots);
    cas.iter()
        .map(|(id, &tickets)| (id.clone(), Rewards::from(tickets) * rewards_per_ticket))
        .collect()
}

fn lottery_rewards(
    funds: Funds,
    cas: &TicketsDistribution,
    rewards_slots: &ProposalRewardSlots,
) -> CaRewards {
    let rewards_per_ticket = funds / Funds::from(rewards_slots.filled_slots);
    let lottery_winnings = lottery::lottery_distribution(cas, rewards_slots.filled_slots);
    lottery_winnings
        .into_iter()
        .map(|(ca, tickets_won)| (ca, Rewards::from(tickets_won) * rewards_per_ticket))
        .collect()
}

fn calculate_ca_rewards_for_proposal(
    proposal_reward: &ProposalReward,
    proposal_reviews: &[AdvisorReviewRow],
    rewards_slots: &ProposalRewardSlots,
) -> CaRewards {
    let ProposalReward { state, funds } = proposal_reward;
    let tickets_distribution = load_tickets_from_reviews(proposal_reviews, rewards_slots);
    match state {
        ProposalFundsState::Exact | ProposalFundsState::Unfilled(_) => {
            distribute_rewards(*funds, &tickets_distribution, rewards_slots)
        }
        ProposalFundsState::OverLoaded => {
            lottery_rewards(*funds, &tickets_distribution, rewards_slots)
        }
    }
}

pub fn calculate_ca_rewards(
    proposal_reviews: &ProposalsReviews,
    approved_proposals: &ApprovedProposals,
    funding: &FundSetting,
    rewards_slots: &ProposalRewardSlots,
) -> CaRewards {
    let proposal_funds =
        calculate_funds_per_proposal(proposal_reviews, approved_proposals, funding, rewards_slots);

    let mut ca_rewards: CaRewards = proposal_reviews
        .values()
        .flat_map(|reviews| reviews.iter().map(|review| review.assessor.clone()))
        .zip(std::iter::repeat(Rewards::default()))
        .collect();

    for (proposal, reviews) in proposal_reviews {
        let proposal_reward = proposal_funds.get(proposal).unwrap();
        let proposal_rewards =
            calculate_ca_rewards_for_proposal(proposal_reward, reviews, rewards_slots);

        for (ca, rewards) in proposal_rewards {
            *ca_rewards.get_mut(&ca).unwrap() += rewards;
        }
    }

    ca_rewards
}
