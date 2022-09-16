use graphql_client::GraphQLQuery;

#[derive(GraphQLQuery)]
#[graphql(
    query_path = "catalyst-toolbox/resources/explorer/transaction_by_id.graphql",
    schema_path = "catalyst-toolbox/resources/explorer/schema.graphql",
    response_derives = "Debug"
)]
pub struct TransactionById;
