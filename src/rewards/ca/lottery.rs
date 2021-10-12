use super::CommunityAdvisor;
use rand::SeedableRng;
use rand_chacha::{ChaCha8Rng, ChaChaRng};
use std::collections::{BTreeMap, HashMap};

pub type Seed = <ChaChaRng as SeedableRng>::Seed;
pub type TotalTickets = u64;
pub type TicketsDistribution = BTreeMap<CommunityAdvisor, TotalTickets>;
pub type CasWinnings = HashMap<CommunityAdvisor, TotalTickets>;

pub fn lottery_distribution(
    distribution: TicketsDistribution,
    tickets_to_distribute: TotalTickets,
    seed: Seed,
) -> CasWinnings {
    let mut rng = ChaCha8Rng::from_seed(seed);
    let total_tickets = distribution.values().sum::<u64>() as usize;

    // Virtually create all tickets and choose the winning tickets using their index.
    let mut indexes =
        rand::seq::index::sample(&mut rng, total_tickets, tickets_to_distribute as usize)
            .into_vec();
    indexes.sort_unstable();
    let mut indexes = indexes.into_iter();

    // To avoid using too much memory, tickets are not actually created, and we iterate
    // the CAs to reconstruct the owner of each ticket.
    let mut winnings = CasWinnings::new();
    let mut cumulative_ticket_index = 0;

    // Consistent iteration is needed to get reproducible results. In this case,
    // it's ensured by the use of BTreeMap::iter()
    for (ca, n_tickets) in distribution.into_iter() {
        let tickets_won = indexes
            .by_ref()
            .take_while(|winning_tkt| (*winning_tkt as u64) < cumulative_ticket_index + n_tickets)
            .count();
        cumulative_ticket_index += n_tickets;
        winnings.insert(ca, tickets_won as u64);
    }
    winnings
}
