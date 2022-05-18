use graphql_client::GraphQLQuery;
use jormungandr_lib::crypto::account::Identifier;
use std::collections::HashSet;
use thiserror::Error;
use voting_hir::VotingGroup;

pub trait VotingGroupAssigner {
    fn assign(&self, vk: &Identifier) -> VotingGroup;
}

pub struct RepsVotersAssigner {
    direct_voters: VotingGroup,
    reps: VotingGroup,
    repsdb: HashSet<Identifier>,
}

#[derive(Debug, Error)]
pub enum Error {
    #[error(transparent)]
    Io(#[from] reqwest::Error),
}

#[derive(GraphQLQuery)]
#[graphql(
    query_path = "resources/repsdb/all_representatives.graphql",
    schema_path = "resources/repsdb/schema.graphql",
    response_derives = "Debug"
)]
pub struct AllReps;

fn get_all_reps(url: impl reqwest::IntoUrl) -> Result<HashSet<Identifier>, Error> {
    let response: all_reps::ResponseData = reqwest::blocking::Client::new()
        .post(url)
        .json(&AllReps::build_query(all_reps::Variables))
        .send()?
        .json()?;

    Ok(response
        .representatives
        .iter()
        .flat_map(|reps| reps.data.iter())
        .flat_map(|rep| rep.attributes.as_ref())
        .flat_map(|attributes| attributes.address.as_ref())
        .flat_map(|addr| Identifier::from_hex(addr))
        .collect())
}

impl RepsVotersAssigner {
    pub fn new(
        direct_voters: VotingGroup,
        reps: VotingGroup,
        repsdb_url: impl reqwest::IntoUrl,
    ) -> Result<Self, Error> {
        Ok(Self {
            direct_voters,
            reps,
            repsdb: get_all_reps(repsdb_url)?,
        })
    }
}

impl VotingGroupAssigner for RepsVotersAssigner {
    fn assign(&self, vk: &Identifier) -> VotingGroup {
        if self.repsdb.contains(vk) {
            self.reps.clone()
        } else {
            self.direct_voters.clone()
        }
    }
}

#[cfg(test)]
impl<F> VotingGroupAssigner for F
where
    F: Fn(&Identifier) -> VotingGroup,
{
    fn assign(&self, vk: &Identifier) -> VotingGroup {
        self(vk)
    }
}
