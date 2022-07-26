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
        #[clap(short, long, value_enum)]
        environment: Environment,
    },
    /// Sends a message to the DataEdge contract
    SendMessage {
        #[clap(short, long, value_enum)]
        environment: Environment,
        #[clap(value_enum)]
        message: Message,
    },
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    use Tasks::*;
    match Tasks::parse() {
        EncodeMessageSamples => message_samples::encode()?,
        CurrentEpoch { environment } => contracts::current_epoch(environment).await?,
        SendMessage {
            environment,
            message,
        } => contracts::send_message(message, environment).await?,
    };
    Ok(())
}

#[derive(clap::ValueEnum, Clone)]
pub enum Message {
    Reset,
}

#[derive(clap::ValueEnum, Clone)]
pub enum Environment {
    Development,
    Staging,
    Production,
}

impl Environment {
    fn resolve_configuration_path(&self) -> anyhow::Result<PathBuf> {
        let mut base = PathBuf::from("crates//oracle/config/");
        let mid = match self {
            Environment::Development => ("dev"),
            Environment::Staging => ("staging"),
            Environment::Production => ("prod"),
        };
        base.push(mid);
        base.push("config.toml");
        let path = base.canonicalize()?;
        anyhow::ensure!(
            path.exists(),
            "Could not find configuration file at: {path:?}"
        );
        Ok(path)
    }

    fn resolve_config(&self) -> anyhow::Result<Config> {
        let config_path = self.resolve_configuration_path()?;
        let config = Config::parse_from(&[config_path]);
        Ok(config)
    }
}
