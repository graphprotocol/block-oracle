use crate::Config;

pub async fn print_current_epoch(config: Config) -> anyhow::Result<()> {
    let contracts = super::init_contracts(config)?;
    let current_epoch = contracts.query_current_epoch().await?;
    println!("{current_epoch}");
    Ok(())
}
