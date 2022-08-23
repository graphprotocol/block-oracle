use clap::Parser as _;

mod message_samples;

/// Block Oracle automation scripts
#[derive(clap::Parser)]
enum Tasks {
    /// Compile and display encoded message samples
    EncodeMessageSamples {
        #[clap(short, long, action)]
        calldata: bool,
    },
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    use Tasks::*;
    match Tasks::parse() {
        EncodeMessageSamples { calldata } => message_samples::encode(calldata)?,
    };
    Ok(())
}

#[derive(clap::ValueEnum, Clone)]
pub enum Message {
    Reset,
}
