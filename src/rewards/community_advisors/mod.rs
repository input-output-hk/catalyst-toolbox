mod funding;
mod lottery;

use crate::community_advisors::models::{AdvisorReviewRow, ReviewScore};
use lottery::TicketsDistribution;
use rand::{Rng, SeedableRng};
use rand_chacha::{ChaCha8Rng, ChaChaRng};

use std::collections::{HashMap, HashSet};

pub use crate::rewards::community_advisors::funding::ProposalRewardSlots;
pub use funding::{FundSetting, Funds};

pub type Seed = <ChaChaRng as SeedableRng>::Seed;
pub type CommunityAdvisor = String;
pub type ProposalId = String;
// Lets match to the same type as the funds, but naming it funds would be confusing
pub type Rewards = Funds;

pub type CaRewards = HashMap<CommunityAdvisor, Rewards>;
pub type ProposalsReviews = HashMap<ProposalId, Vec<AdvisorReviewRow>>;
pub type ApprovedProposals = HashMap<ProposalId, Funds>;

const LEGACY_max_winning_tickets: u64 = 3;

struct ProposalRewards {
    per_ticket_reward: Rewards,
    tickets: ProposalTickets,
}

enum ProposalTickets {
    Legacy {
        eligible_assessors: HashSet<CommunityAdvisor>,
        winning_tkts: u64,
    },
    Fund6 {
        ticket_distribution: TicketsDistribution,
        winning_tkts: u64,
    },
}

fn calculate_rewards_per_proposal(
    proposal_reviews: ProposalsReviews,
    approved_proposals: &ApprovedProposals,
    funding: &FundSetting,
    rewards_slots: &ProposalRewardSlots,
) -> Vec<ProposalRewards> {
    let bonus_funds = funding.bonus_funds();

    let mut total_tickets = 0;
    let total_approved_budget = approved_proposals.values().sum::<Funds>();
    let mut proposals_tickets = Vec::new();

    for (id, reviews) in proposal_reviews {
        let filtered = reviews
            .into_iter()
            .filter(|review| !matches!(review.score(), ReviewScore::FilteredOut))
            .collect::<Vec<_>>();
        let tickets = load_tickets_from_reviews(&filtered, rewards_slots);
        match tickets {
            ProposalTickets::Legacy { winning_tkts, .. } => {
                // it would be a bit harder to track it otherwise, and we don't need this additional
                // complexity now
                assert_eq!(
                    0,
                    rewards_slots.max_winning_tickets() % LEGACY_max_winning_tickets
                );
                total_tickets += winning_tkts
                    * (rewards_slots.max_winning_tickets() / LEGACY_max_winning_tickets);
            }
            ProposalTickets::Fund6 { winning_tkts, .. } => total_tickets += winning_tkts,
        }

        proposals_tickets.push((id, tickets));
    }

    let base_ticket_reward = funding.proposal_funds() / Rewards::from(total_tickets);

    proposals_tickets
        .into_iter()
        .map(|(id, tickets)| {
            let bonus_reward = approved_proposals
                .get(&id)
                .map(|budget| bonus_funds * budget / total_approved_budget)
                .unwrap_or_default();
            let per_ticket_reward = match tickets {
                ProposalTickets::Legacy { winning_tkts, .. } => {
                    base_ticket_reward * Rewards::from(rewards_slots.max_winning_tickets())
                        / Rewards::from(LEGACY_max_winning_tickets)
                        + bonus_reward / Rewards::from(winning_tkts)
                }
                ProposalTickets::Fund6 { winning_tkts, .. } => {
                    base_ticket_reward + bonus_reward / Rewards::from(winning_tkts)
                }
            };
            ProposalRewards {
                tickets,
                per_ticket_reward,
            }
        })
        .collect()
}

fn load_tickets_from_reviews(
    proposal_reviews: &[AdvisorReviewRow],
    rewards_slots: &ProposalRewardSlots,
) -> ProposalTickets {
    let is_legacy = proposal_reviews
        .iter()
        .any(|rev| matches!(rev.score(), ReviewScore::NA));

    if is_legacy {
        return ProposalTickets::Legacy {
            eligible_assessors: proposal_reviews
                .iter()
                .map(|rev| rev.assessor.clone())
                .collect(),
            winning_tkts: std::cmp::min(proposal_reviews.len() as u64, LEGACY_max_winning_tickets),
        };
    }

    let excellent_reviews = proposal_reviews
        .iter()
        .filter(|rev| matches!(rev.score(), ReviewScore::Excellent))
        .count();
    let good_reviews = proposal_reviews
        .iter()
        .filter(|rev| matches!(rev.score(), ReviewScore::Good))
        .count();

    // assumes only one review per assessor in a single proposal
    let ticket_distribution = proposal_reviews
        .iter()
        .map(|rev| {
            let tickets = match rev.score() {
                ReviewScore::Excellent => rewards_slots.excellent_slots,
                ReviewScore::Good => rewards_slots.good_slots,
                _ => unreachable!("we've already filtered out other review scores"),
            };

            (rev.assessor.clone(), tickets)
        })
        .collect();

    let excellent_winning_tkts = std::cmp::min(
        excellent_reviews as u64,
        rewards_slots.max_excellent_reviews,
    ) * rewards_slots.excellent_slots;
    let good_winning_tkts = std::cmp::min(good_reviews as u64, rewards_slots.max_good_reviews)
        * rewards_slots.good_slots;

    ProposalTickets::Fund6 {
        ticket_distribution,
        winning_tkts: excellent_winning_tkts + good_winning_tkts,
    }
}

fn calculate_ca_rewards_for_proposal<R: Rng>(
    proposal_reward: ProposalRewards,
    rng: &mut R,
) -> CaRewards {
    let ProposalRewards {
        tickets,
        per_ticket_reward,
    } = proposal_reward;

    let (tickets_distribution, tickets_to_distribute) = match tickets {
        ProposalTickets::Fund6 {
            tickets_distribution,
            winning_tkts,
        } => (distribution, winning_tkts),
        ProposalTickets::Legacy {
            eligible_assessors,
            winning_tkts,
        } => (
            eligible_assessors.into_iter().map(|ca| (ca, 1)).collect(),
            winning_tkts,
        ),
    };

    lottery::lottery_distribution(tickets_distribution, tickets_to_distribute, rng)
        .into_iter()
        .map(|(ca, tickets_won)| (ca, Rewards::from(tickets_won) * per_ticket_reward))
        .collect()
}

pub fn calculate_ca_rewards(
    proposal_reviews: ProposalsReviews,
    approved_proposals: &ApprovedProposals,
    funding: &FundSetting,
    rewards_slots: &ProposalRewardSlots,
    seed: Seed,
) -> CaRewards {
    let proposal_rewards = calculate_rewards_per_proposal(
        proposal_reviews,
        approved_proposals,
        funding,
        rewards_slots,
    );
    let mut ca_rewards = CaRewards::new();

    let mut rng = ChaCha8Rng::from_seed(seed);

    for proposal_reward in proposal_rewards {
        let rewards = calculate_ca_rewards_for_proposal(proposal_reward, &mut rng);

        for (ca, rewards) in rewards {
            *ca_rewards.entry(ca).or_insert(Rewards::ZERO) += rewards;
        }
    }

    ca_rewards
}
