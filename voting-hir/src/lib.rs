use jormungandr_lib::{crypto::account::Identifier, interfaces::Value};

pub type VotingGroup = String;

/// Define High Level Intermediate Representation (HIR) for voting
/// entities in the Catalyst ecosystem.
///
/// This is intended as a high level description of the setup, which is not
/// enough on its own to spin a blockchain, but it's slimmer, easier to understand
/// and free from implementation constraints.
///
/// You can roughly read this as
/// "voting_key will participate in this voting round with role voting_group and will have voting_power influence"
pub struct VotingHIR {
    pub voting_key: Identifier,
    /// Voting group this key belongs to.
    /// If this key belong to multiple voting groups, multiple records for the same
    /// key will be used.
    pub voting_group: VotingGroup,
    /// Voting power as processed by the snapshot
    pub voting_power: Value,
}
