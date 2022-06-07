use clap::Parser;
use epoch_encoding as ee;
use serde::{Deserialize, Deserializer, Serialize};
use std::{collections::HashMap, io};

#[derive(Parser)]
#[clap(name = "oracle-encoder")]
#[clap(bin_name = "oracle-encoder")]
#[clap(author, version, about, long_about = None)]
struct OracleEncoder {
    #[clap(long)]
    json_path: String,
}

fn main() -> io::Result<()> {
    let inputs = OracleEncoder::parse();

    let file_contents = std::fs::read_to_string(inputs.json_path)?;
    let message_blocks: Vec<MessageBlock> = serde_json::from_str(&file_contents).unwrap();

    let mut encoded_message_blocks = vec![];
    for block in message_blocks {
        let contents = match block {
            MessageBlock::MessageBlock(b) => b,
            MessageBlock::MessageBlockWithOneMessage(m) => vec![m],
        };
        let mut message_types = vec![];
        let mut compressed_contents = vec![];
        for message in contents {
            let (message_type, ready_to_encode) = match message {
                Message::Reset => ("Reset", ee::CompressedMessage::Reset),
                Message::CorrectEpochs {} => (
                    "CorrectEpochs",
                    ee::CompressedMessage::CorrectEpochs {
                        data_by_network_id: HashMap::new(),
                    },
                ),
                Message::UpdateVersion { version_number } => ("UpdateVersion", {
                    ee::CompressedMessage::UpdateVersion { version_number }
                }),
                Message::RegisterNetworks { remove, add } => ("RegisterNetworks", {
                    ee::CompressedMessage::RegisterNetworks { remove, add }
                }),
                Message::SetBlockNumbersForNextEpoch(SetBlockNumbersForNextEpoch::Empty {
                    count,
                }) => ("SetBlockNumbersForNextEpoch", {
                    ee::CompressedMessage::SetBlockNumbersForNextEpoch(
                        ee::CompressedSetBlockNumbersForNextEpoch::Empty { count },
                    )
                }),
                Message::SetBlockNumbersForNextEpoch(SetBlockNumbersForNextEpoch::NonEmpty {
                    merkle_root,
                    accelerations,
                }) => (
                    "SetBlockNumbersForNextEpoch",
                    ee::CompressedMessage::SetBlockNumbersForNextEpoch(
                        ee::CompressedSetBlockNumbersForNextEpoch::NonEmpty {
                            root: merkle_root
                                .try_into()
                                .expect("Bad JSON: The Merkle root must have exactly 32 bytes."),
                            accelerations,
                        },
                    ),
                ),
            };
            message_types.push(message_type);
            compressed_contents.push(ready_to_encode);
        }
        let mut payload = Vec::new();
        ee::serialize_messages(&compressed_contents[..], &mut payload);
        encoded_message_blocks.push((message_types, payload));
    }

    for (i, (message_types, block_payload)) in encoded_message_blocks.iter().enumerate() {
        println!(
            "{} ({}): 0x{}",
            i + 1,
            message_types.join(", "),
            hex::encode(block_payload)
        );
    }

    Ok(())
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
