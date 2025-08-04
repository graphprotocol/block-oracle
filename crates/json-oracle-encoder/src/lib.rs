use anyhow::anyhow;
use epoch_encoding as ee;
use ethabi::{encode, short_signature, ParamType, Token};
use serde::{Deserialize, Deserializer, Serialize};
use std::collections::BTreeMap;

type EncodedMessageBlocks = Vec<(Vec<&'static str>, Vec<u8>)>;

pub fn messages_to_payload(json: serde_json::Value) -> anyhow::Result<Vec<u8>> {
    let encoded_message_blocks = messages_to_encoded_message_blocks(json)?;
    if encoded_message_blocks.len() != 1 {
        return Err(anyhow!("expected exactly one message block"));
    }
    Ok(encoded_message_blocks[0].1.clone())
}

pub fn messages_to_calldata(json: serde_json::Value) -> anyhow::Result<Vec<u8>> {
    let encoded_message_blocks = messages_to_encoded_message_blocks(json)?;
    if encoded_message_blocks.len() != 1 {
        return Err(anyhow!("expected exactly one message block"));
    }
    let calldata = calldata(encoded_message_blocks[0].1.clone());
    Ok(calldata)
}

pub fn print_encoded_json_messages(
    output_kind: OutputKind,
    json: serde_json::Value,
) -> anyhow::Result<()> {
    let encoded_message_blocks = messages_to_encoded_message_blocks(json)?;

    match output_kind {
        OutputKind::Calldata => {
            for (_, block_payload) in encoded_message_blocks.into_iter() {
                let calldata = calldata(block_payload);
                println!("{}", hex::encode(calldata));
            }
        }
        OutputKind::Payload => {
            for (i, (message_types, block_payload)) in encoded_message_blocks.iter().enumerate() {
                println!(
                    "{} ({}): 0x{}",
                    i + 1,
                    message_types.join(", "),
                    hex::encode(block_payload)
                );
            }
        }
    }

    Ok(())
}

fn messages_to_encoded_message_blocks(
    json: serde_json::Value,
) -> anyhow::Result<EncodedMessageBlocks> {
    let message_blocks: Vec<MessageBlock> = serde_json::from_value(json)?;

    let mut encoded_message_blocks = vec![];
    for block in message_blocks {
        let contents = match block {
            MessageBlock::MessageBlock(b) => b,
            MessageBlock::MessageBlockWithOneMessage(m) => vec![m],
        };
        let mut message_types = vec![];
        let mut compressed_contents = vec![];
        for message in contents {
            let message_type = message.message_type();
            let ready_to_encode = match message {
                Message::Reset => ee::CompressedMessage::Reset,
                Message::CorrectEpochs {} => ee::CompressedMessage::CorrectEpochs {
                    data_by_network_id: BTreeMap::new(),
                },
                Message::UpdateVersion { version_number } => {
                    ee::CompressedMessage::UpdateVersion { version_number }
                }
                Message::RegisterNetworks { remove, add } => {
                    ee::CompressedMessage::RegisterNetworks { remove, add }
                }
                Message::RegisterNetworksAndAliases { remove, add } => {
                    ee::CompressedMessage::RegisterNetworksAndAliases { remove, add }
                }
                Message::ChangePermissions {
                    address,
                    valid_through,
                    permissions,
                } => ee::CompressedMessage::ChangePermissions {
                    address: address
                        .try_into()
                        .map_err(|_| anyhow!("Bad address length; must be 20 bytes"))?,
                    valid_through,
                    permissions: permissions
                        .into_iter()
                        .map(|x| ee::Message::str_to_u64(x.as_str()))
                        .collect(),
                },
                Message::SetBlockNumbersForNextEpoch(SetBlockNumbersForNextEpoch::Empty {
                    count,
                }) => ee::CompressedMessage::SetBlockNumbersForNextEpoch(
                    ee::CompressedSetBlockNumbersForNextEpoch::Empty { count },
                ),
                Message::SetBlockNumbersForNextEpoch(SetBlockNumbersForNextEpoch::NonEmpty {
                    merkle_root,
                    accelerations,
                }) => ee::CompressedMessage::SetBlockNumbersForNextEpoch(
                    ee::CompressedSetBlockNumbersForNextEpoch::NonEmpty {
                        root: merkle_root.try_into().map_err(|_| {
                            anyhow!("Bad JSON: The Merkle root must have exactly 32 bytes.")
                        })?,
                        accelerations,
                    },
                ),
                Message::CorrectLastEpoch {
                    chain_id,
                    block_number,
                    merkle_root,
                } => ee::CompressedMessage::CorrectLastEpoch {
                    chain_id,
                    block_number,
                    merkle_root: merkle_root.try_into().map_err(|_| {
                        anyhow!("Bad JSON: The Merkle root must have exactly 32 bytes.")
                    })?,
                },
            };
            message_types.push(message_type);
            compressed_contents.push(ready_to_encode);
        }
        let mut payload = Vec::new();
        ee::serialize_messages(&compressed_contents[..], &mut payload);
        encoded_message_blocks.push((message_types, payload));
    }

    Ok(encoded_message_blocks)
}

/// Whether the JSON encoder should output the payload of the compressed messages, or the full
/// calldata.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputKind {
    Calldata,
    Payload,
}

#[derive(Serialize, Deserialize)]
#[serde(untagged)]
pub enum MessageBlock {
    MessageBlock(Vec<Message>),
    MessageBlockWithOneMessage(Message),
}

#[derive(Serialize, Deserialize)]
#[serde(tag = "message")]
#[serde(rename_all = "PascalCase")]
pub enum Message {
    SetBlockNumbersForNextEpoch(SetBlockNumbersForNextEpoch),
    CorrectEpochs {
        // TODO.
    },
    #[serde(rename_all = "camelCase")]
    RegisterNetworks {
        remove: Vec<u64>,
        add: Vec<String>,
    },
    #[serde(rename_all = "camelCase")]
    UpdateVersion {
        version_number: u64,
    },
    Reset,
    RegisterNetworksAndAliases {
        remove: Vec<u64>,
        add: Vec<(String, String)>,
    },
    ChangePermissions {
        #[serde(deserialize_with = "deserialize_hex")]
        address: Vec<u8>,
        valid_through: u64,
        permissions: Vec<String>,
    },
    #[serde(rename_all = "camelCase")]
    CorrectLastEpoch {
        chain_id: String,
        block_number: u64,
        #[serde(deserialize_with = "deserialize_hex")]
        merkle_root: Vec<u8>,
    },
}

impl Message {
    pub const fn message_type(&self) -> &'static str {
        match self {
            Message::SetBlockNumbersForNextEpoch(_) => "SetBlockNumbersForNextEpoch",
            Message::CorrectEpochs { .. } => "CorrectEpochs",
            Message::RegisterNetworks { .. } => "RegisterNetworks",
            Message::UpdateVersion { .. } => "UpdateVersion",
            Message::Reset => "Reset",
            Message::RegisterNetworksAndAliases { .. } => "RegisterNetworksAndAliases",
            Message::ChangePermissions { .. } => "ChangePermissions",
            Message::CorrectLastEpoch { .. } => "CorrectLastEpoch",
        }
    }
}

#[derive(Serialize, Deserialize)]
#[serde(untagged)]
#[serde(rename_all = "camelCase")]
pub enum SetBlockNumbersForNextEpoch {
    #[serde(rename_all = "camelCase")]
    Empty { count: u64 },
    #[serde(rename_all = "camelCase")]
    NonEmpty {
        #[serde(deserialize_with = "deserialize_hex")]
        merkle_root: Vec<u8>,
        accelerations: Vec<i64>,
    },
}

fn deserialize_hex<'de, D>(deserializer: D) -> Result<Vec<u8>, D::Error>
where
    D: Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;
    hex::decode(s.strip_prefix("0x").unwrap_or(s.as_str())).map_err(serde::de::Error::custom)
}

pub fn calldata(payload: Vec<u8>) -> Vec<u8> {
    let signature = short_signature("crossChainEpochOracle", &[ParamType::Bytes]);
    let payload = Token::Bytes(payload);
    let encoded = encode(&[payload]);
    signature.into_iter().chain(encoded).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_correct_last_epoch_json_encoding() {
        let json_str = r#"[
            {
                "message": "CorrectLastEpoch",
                "chainId": "eip155:1",
                "blockNumber": 12345678,
                "merkleRoot": "0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef"
            }
        ]"#;

        let json: serde_json::Value = serde_json::from_str(json_str).unwrap();
        let payload = messages_to_payload(json).unwrap();

        // Verify payload is not empty
        assert!(!payload.is_empty());

        // First byte should contain tag 7 in lower nibble
        assert_eq!(payload[0] & 0x0F, 7);
    }

    #[test]
    fn test_correct_last_epoch_with_multiple_messages() {
        let json_str = r#"[
            [
                {
                    "message": "RegisterNetworks",
                    "remove": [],
                    "add": ["eip155:1"]
                },
                {
                    "message": "CorrectLastEpoch",
                    "chainId": "eip155:1",
                    "blockNumber": 99999,
                    "merkleRoot": "0xabcdefabcdefabcdefabcdefabcdefabcdefabcdefabcdefabcdefabcdefabcd"
                }
            ]
        ]"#;

        let json: serde_json::Value = serde_json::from_str(json_str).unwrap();
        let encoded_blocks = messages_to_encoded_message_blocks(json).unwrap();

        assert_eq!(encoded_blocks.len(), 1);
        let (message_types, payload) = &encoded_blocks[0];

        // Verify message types
        assert_eq!(message_types.len(), 2);
        assert_eq!(message_types[0], "RegisterNetworks");
        assert_eq!(message_types[1], "CorrectLastEpoch");

        // Verify payload contains both messages
        // Preamble should have tags 3 and 7
        let preamble = payload[0];
        assert_eq!(preamble & 0x0F, 3); // First tag
        assert_eq!((preamble >> 4) & 0x0F, 7); // Second tag
    }

    #[test]
    fn test_correct_last_epoch_invalid_merkle_root() {
        // Test with invalid merkle root (not 32 bytes)
        let json_str = r#"[
            {
                "message": "CorrectLastEpoch",
                "chainId": "eip155:1",
                "blockNumber": 12345678,
                "merkleRoot": "0x1234"
            }
        ]"#;

        let json: serde_json::Value = serde_json::from_str(json_str).unwrap();
        let result = messages_to_payload(json);

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("32 bytes"));
    }
}
