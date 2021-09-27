use rand::{
    distributions::{Distribution, WeightedIndex},
    thread_rng,
};
use std::collections::HashMap;

type Ca = String;
type TotalTickets = usize;
type TicketsDistribution = HashMap<Ca, TotalTickets>;

pub fn winner(distribution: &TicketsDistribution) -> Ca {
    let mut rng = thread_rng();
    let items: Vec<(Ca, TotalTickets)> = distribution
        .iter()
        .map(|(ca, &tickets)| (ca.clone(), tickets))
        .collect();
    let mut dist = WeightedIndex::new(items.iter().map(|x| x.1)).unwrap();
    items[dist.sample(&mut rng)].0.clone()
}
