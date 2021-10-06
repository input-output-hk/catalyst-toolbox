use super::Ca;
use rand::{
    distributions::{Distribution, WeightedIndex},
    thread_rng,
};
use std::collections::HashMap;

pub type TotalTickets = u64;
pub type TicketsDistribution = HashMap<Ca, TotalTickets>;
pub type CasWinnings = HashMap<Ca, TotalTickets>;

pub fn lottery_winner(distribution: &TicketsDistribution) -> Ca {
    let mut rng = thread_rng();
    let items: Vec<(Ca, TotalTickets)> = distribution
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
    let mut winnings: CasWinnings = distribution
        .keys()
        .cloned()
        .zip(std::iter::repeat(0))
        .collect();

    for _ in 0..tickets_to_distribute {
        let winner = lottery_winner(distribution);
        // we build a default winnings hashmap from the original distribution keys so it is ensured
        // that the key will be present
        *winnings.get_mut(&winner).unwrap() += 1;
    }

    winnings
}
