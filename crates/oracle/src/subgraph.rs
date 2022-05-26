use crate::{Config, SubgraphApi};
use async_trait::async_trait;
use graphql_client::{GraphQLQuery, Response};
use serde::{de, Deserialize, Deserializer};
use std::fmt;
use url::Url;

pub type Id = String;
pub type BigInt = u128;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "src/graphql/schema.graphql",
    query_path = "src/graphql/query.graphql",
    response_derives = "Debug",
    variables_derives = "Debug",
    deprecated = "warn"
)]
pub struct SubgraphState;

pub async fn query(url: &str) -> reqwest::Result<subgraph_state::ResponseData> {
    // TODO: authentication token.
    let client = reqwest::Client::builder()
        .user_agent("block-oracle")
        .build()
        .unwrap();
    let request_body = SubgraphState::build_query(subgraph_state::Variables);
    let request = client.post(url).json(&request_body);
    let response = request.send().await?;
    let response_body: Response<subgraph_state::ResponseData> = response.json().await?;

    println!("{:?}", response_body.errors);

    Ok(response_body.data.unwrap())
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

#[cfg(test)]
mod tests {
    use super::*;
}
