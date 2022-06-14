use clap::Parser;

mod message_samples;

/// Block Oracle automation scripts
#[derive(Parser)]
#[clap()]
enum Tasks {
    /// Compile and display encoded message samples
    EncodeMessageSamples,
}

fn main() -> anyhow::Result<()> {
    match Tasks::parse() {
        Tasks::EncodeMessageSamples => message_samples::encode()?,
    };
    Ok(())
}
