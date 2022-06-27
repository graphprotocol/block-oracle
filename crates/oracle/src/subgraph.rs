use crate::error_handling::format_slice;
use crate::{Config, SubgraphApi};
use async_trait::async_trait;
use graphql_client::{GraphQLQuery, Response};
use itertools::Itertools;
use reqwest::Url;
use serde::{de, Deserialize, Deserializer};
use std::collections::HashSet;
use std::fmt;

pub type Id = String;
pub type BigInt = u128;

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
pub struct SubgraphState;

pub struct SubgraphQuery {
    url: Url,
}

impl From<&Config> for SubgraphQuery {
    fn from(config: &Config) -> Self {
        Self {
            url: config.subgraph_url.clone(),
        }
    }
}

#[async_trait]
impl SubgraphApi for SubgraphQuery {
    type State = subgraph_state::SubgraphStateGlobalState;
    type Error = anyhow::Error;

    async fn get_subgraph_state(&self) -> anyhow::Result<Option<Self::State>> {
        Ok(query(self.url.clone()).await?)
    }
}

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

pub async fn query(
    url: Url,
) -> Result<Option<subgraph_state::SubgraphStateGlobalState>, SubgraphQueryError> {
    // TODO: authentication token.
    let client = reqwest::Client::builder()
        .user_agent("block-oracle")
        .build()
        .unwrap();
    let request_body = SubgraphState::build_query(subgraph_state::Variables);
    let request = client.post(url).json(&request_body);
    let response = request.send().await?;
    let response_body: Response<subgraph_state::ResponseData> = response.json().await?;
    match response_body.errors.as_deref() {
        None | Some(&[]) => {
            // Unwrap: We just checked that there are no errors
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
        Some([e]) if e.message == "indexing_error" => Err(SubgraphQueryError::IndexingError),
        Some(errs) => Err(SubgraphQueryError::Other {
            messages: errs.into_iter().map(|e| e.message.clone()).collect(),
        }),
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DataEdge {
    id: Id,
    #[serde(deserialize_with = "deserialize_hex_string")]
    owner: Vec<u8>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Payload {
    id: Id,
    #[serde(deserialize_with = "deserialize_hex_string")]
    data: Vec<u8>,
    submitter: String,
    message_blocks: Vec<MessageBlock>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MessageBlock {
    id: Id,
    #[serde(deserialize_with = "deserialize_hex_string")]
    data: Vec<u8>,
    paylaod: Payload,
    messages: Vec<Message>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MessageData {
    id: Id,
    block: MessageBlock,
    data: Option<Vec<u8>>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Message {
    data: MessageData,
    kind: MessageKind,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum MessageKind {
    SetBlockNumbersForEpochMessage(SetBlockNumbersForEpochMessage),
    CorrectEpochsMessage(CorrectEpochsMessage),
    UpdateVersionsMessage(UpdateVersionsMessage),
    RegisterNetworksMessage(RegisterNetworksMessage),
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SetBlockNumbersForEpochMessage {
    merkle_root: Option<Vec<u8>>,
    accelerations: Option<Vec<u128>>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CorrectEpochsMessage {}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateVersionsMessage {}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RegisterNetworksMessage {
    remove_count: u64,
    add_count: u64,
    networks_removed: Vec<Network>,
    networks_added: Vec<Network>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Network {
    id: Id,
    chain_id: String,
    block_numbers: Vec<NetworkEpochBlockNumber>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NetworkEpochBlockNumber {
    id: Id,
    acceleration: i128,
    delta: i128,
    block_number: u128,
    network: Network,
    epoch: Epoch,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Epoch {
    id: Id,
    epoch_number: u128,
    block_numbers: Vec<NetworkEpochBlockNumber>,
}

fn deserialize_hex_string<'de, D>(de: D) -> Result<Vec<u8>, D::Error>
where
    D: Deserializer<'de>,
{
    struct HexStringVisitor;

    impl<'de> de::Visitor<'de> for HexStringVisitor {
        type Value = Vec<u8>;

        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            formatter.write_str("a hexadecimal string (e.g. 0x1337)")
        }

        fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            if let Some(s) = v.strip_prefix("0x") {
                hex::decode(s).map_err(de::Error::custom)
            } else {
                Err(de::Error::custom("not a hexadecimal string"))
            }
        }
    }

    de.deserialize_string(HexStringVisitor)
}
