use crate::error_handling::format_slice;
use crate::SubgraphApi;
use async_trait::async_trait;
use graphql_client::{GraphQLQuery, Response};
use itertools::Itertools;
use reqwest::Url;
use std::collections::HashSet;

pub struct SubgraphQuery {
    url: Url,
}

impl SubgraphQuery {
    pub fn new(url: Url) -> Self {
        Self { url }
    }
}

#[async_trait]
impl SubgraphApi for SubgraphQuery {
    type State = subgraph_state::SubgraphStateGlobalState;
    type Error = anyhow::Error;

    async fn get_subgraph_state(&self) -> anyhow::Result<Option<Self::State>> {
        // TODO: authentication token.
        let client = reqwest::Client::builder()
            .user_agent("block-oracle")
            .build()
            .unwrap();
        let request_body = SubgraphState::build_query(subgraph_state::Variables);
        let request = client.post(self.url.clone()).json(&request_body);
        let response = request.send().await?;
        let response_body: Response<subgraph_state::ResponseData> = response.json().await?;
        match response_body.errors.as_deref() {
            Some([e]) if e.message == "indexing_error" => Err(SubgraphQueryError::IndexingError)?,
            Some(errs) => Err(SubgraphQueryError::Other {
                messages: errs.into_iter().map(|e| e.message.clone()).collect(),
            })?,
            _ => {}
        }

        // Unwrap: We just checked that there are no errors.
        let data = response_body
            .data
            .expect("expected data in the GraphQL query response, but got none");

        if let Some(global_state) = data.global_state {
            validate_subgraph_state(&global_state)?;
            Ok(Some(global_state))
        } else {
            // Subgraph is in a initial state and has no GlobalState yet
            Ok(None)
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum SubgraphQueryError {
    #[error(transparent)]
    Transport(#[from] reqwest::Error),
    #[error("The subgraph is in a failed state")]
    IndexingError,
    #[error("Found duplicated network ids in subgraph state")]
    DuplicatedNetworkIds(HashSet<String>),
    #[error("Found duplicated network indices in subgraph state")]
    DuplicatedNetworkIndices(HashSet<i64>),
    #[error("Found a registered network without an index")]
    NetworkWithMissingIndex(String),
    #[error("Unknown subgraph error(s): {}", format_slice(messages))]
    Other { messages: Vec<String> },
}

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "src/graphql/schema.graphql",
    query_path = "src/graphql/query.graphql",
    response_derives = "Debug,Clone",
    variables_derives = "Debug,Clone",
    deprecated = "warn"
)]
struct SubgraphState;

fn validate_subgraph_state(
    state: &subgraph_state::SubgraphStateGlobalState,
) -> Result<(), SubgraphQueryError> {
    // 1. Validate against  duplicate chain ids (keys)
    let duplicate_network_ids: HashSet<_> =
        state.networks.iter().map(|a| &a.id).duplicates().collect();
    if !duplicate_network_ids.is_empty() {
        let duplicates = duplicate_network_ids.into_iter().cloned().collect();
        return Err(SubgraphQueryError::DuplicatedNetworkIds(duplicates));
    }
    // 2. Validate against duplicate array indices
    // Indices are wrapped in Options, so we must unpack them before checking for duplicates
    let mut unpacked_indices = vec![];
    for network in state.networks.iter() {
        match network.array_index {
            Some(index) => unpacked_indices.push(index),
            None => {
                return Err(SubgraphQueryError::NetworkWithMissingIndex(
                    network.id.clone(),
                ))
            }
        }
    }
    let duplicate_network_indices: HashSet<_> =
        unpacked_indices.iter().copied().duplicates().collect();
    if !duplicate_network_indices.is_empty() {
        return Err(SubgraphQueryError::DuplicatedNetworkIndices(
            duplicate_network_indices,
        ));
    }
    Ok(())
}
