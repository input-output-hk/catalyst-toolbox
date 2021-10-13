mod funding;
mod lottery;

use crate::community_advisors::models::{AdvisorReviewRow, ReviewScore};
use lottery::TicketsDistribution;

use std::collections::HashMap;

pub use crate::rewards::community_advisors::funding::ProposalRewardSlots;
pub use crate::rewards::community_advisors::lottery::Seed;
pub use funding::{FundSetting, Funds};

pub type CommunityAdvisor = String;
pub type ProposalId = String;
// Lets match to the same type as the funds, but naming it funds would be confusing
pub type Rewards = Funds;

pub type CaRewards = HashMap<CommunityAdvisor, Rewards>;
pub type ProposalsReviews = HashMap<ProposalId, Vec<AdvisorReviewRow>>;
pub type ApprovedProposals = HashMap<ProposalId, Funds>;

struct TicketRewards {
    base_ticket_reward: Rewards,
    per_proposal_bonus_reward: HashMap<ProposalId, Rewards>,
}

fn calculate_rewards_per_ticket(
    proposal_reviews: &ProposalsReviews,
    approved_proposals: &ApprovedProposals,
    funding: &FundSetting,
    rewards_slots: &ProposalRewardSlots,
) -> TicketRewards {
    let bonus_funds = funding.bonus_funds();

    let mut total_tickets = 0;
    let total_approved_budget = approved_proposals.values().sum::<Funds>();

    let mut per_proposal_bonus_reward = HashMap::new();

    for (id, reviews) in proposal_reviews {
        let filled_slots = reviews
            .iter()
            .map(|review| match review.score() {
                ReviewScore::Excellent => rewards_slots.excellent_slots,
                ReviewScore::Good => rewards_slots.good_slots,
                _ => unimplemented!(),
            })
            .sum::<u64>();

        // at most we reward the maximum allowed tickets per proposal
        total_tickets += std::cmp::min(filled_slots, rewards_slots.filled_slots);

        if let Some(proposal_budget) = approved_proposals.get(id) {
            per_proposal_bonus_reward.insert(
                id.clone(),
                bonus_funds * proposal_budget / total_approved_budget,
            );
        }
    }

    let base_ticket_reward = funding.proposal_funds() / Rewards::from(total_tickets);
    TicketRewards {
        base_ticket_reward,
        per_proposal_bonus_reward,
    }
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
            _ => unimplemented!(),
        };
        *entry += tickets_to_add;
    }
    tickets_distribution
}

#[allow(clippy::ptr_arg)]
fn calculate_ca_rewards_for_proposal(
    proposal: &ProposalId,
    proposal_reward: &TicketRewards,
    proposal_reviews: &[AdvisorReviewRow],
    rewards_slots: &ProposalRewardSlots,
    seed: Seed,
) -> CaRewards {
    let tickets_distribution = load_tickets_from_reviews(proposal_reviews, rewards_slots);
    let total_proposal_tickets = tickets_distribution.values().sum();
    let tickets_to_distribute = std::cmp::min(rewards_slots.filled_slots, total_proposal_tickets);
    let reward_per_ticket =
        if let Some(bonus) = proposal_reward.per_proposal_bonus_reward.get(proposal) {
            proposal_reward.base_ticket_reward + bonus / Rewards::from(total_proposal_tickets)
        } else {
            proposal_reward.base_ticket_reward
        };
    lottery::lottery_distribution(tickets_distribution, tickets_to_distribute, seed)
        .into_iter()
        .map(|(ca, tickets_won)| (ca, Rewards::from(tickets_won) * reward_per_ticket))
        .collect()
}

pub fn calculate_ca_rewards(
    proposal_reviews: &ProposalsReviews,
    approved_proposals: &ApprovedProposals,
    funding: &FundSetting,
    rewards_slots: &ProposalRewardSlots,
    seed: Seed,
) -> CaRewards {
    let ticket_rewards =
        calculate_rewards_per_ticket(proposal_reviews, approved_proposals, funding, rewards_slots);
    let mut ca_rewards = CaRewards::new();

    for (proposal, reviews) in proposal_reviews {
        let proposal_rewards = calculate_ca_rewards_for_proposal(
            proposal,
            &ticket_rewards,
            reviews,
            rewards_slots,
            seed,
        );

        for (ca, rewards) in proposal_rewards {
            *ca_rewards.entry(ca).or_insert(Rewards::ZERO) += rewards;
        }
    }

    ca_rewards
}
