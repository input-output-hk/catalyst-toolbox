use super::CommunityAdvisor;
use rand::{
    distributions::{Distribution, WeightedIndex},
    thread_rng,
};
use std::collections::HashMap;

pub type TotalTickets = u64;
pub type TicketsDistribution = HashMap<CommunityAdvisor, TotalTickets>;
pub type CasWinnings = HashMap<CommunityAdvisor, TotalTickets>;

pub fn lottery_winner(distribution: &TicketsDistribution) -> CommunityAdvisor {
    let mut rng = thread_rng();
    let items: Vec<(CommunityAdvisor, TotalTickets)> = distribution
        .iter()
        .map(|(ca, &tickets)| (ca.clone(), tickets))
        .collect();
    let dist = WeightedIndex::new(items.iter().map(|x| x.1)).unwrap();
    items[dist.sample(&mut rng)].0.clone()
}

pub fn lottery_distribution(
    distribution: &TicketsDistribution,
    tickets_to_distribute: TotalTickets,
) -> CasWinnings {
    let mut winnings: CasWinnings = CasWinnings::new();

    for _ in 0..tickets_to_distribute {
        let winner = lottery_winner(distribution);
        *winnings.entry(winner).or_insert(0) += 1
    }

    winnings
}
