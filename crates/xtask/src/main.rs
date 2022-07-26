use block_oracle::config::Config;
use clap::Parser as _;
use std::path::PathBuf;

mod contracts;
mod message_samples;

/// Block Oracle automation scripts
#[derive(clap::Parser)]
enum Tasks {
    /// Compile and display encoded message samples
    EncodeMessageSamples,
    /// Queries the Epoch Manager for the current epoch
    CurrentEpoch {
        #[clap(short, long)]
        config_file: PathBuf,
    },
    /// Sends a message to the DataEdge contract
    SendMessage {
        #[clap(short, long)]
        config_file: PathBuf,
        #[clap(value_enum)]
        message: Message,
    },
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    use Tasks::*;
    match Tasks::parse() {
        EncodeMessageSamples => message_samples::encode()?,
        CurrentEpoch { config_file } => {
            let config = Config::parse_from(&[config_file]);
            contracts::current_epoch(config).await?
        }
        SendMessage {
            config_file,
            message,
        } => {
            let config = Config::parse_from(&[config_file]);
            contracts::send_message(message, config).await?
        }
    };
    Ok(())
}

#[derive(clap::ValueEnum, Clone)]
pub enum Message {
    Reset,
}
