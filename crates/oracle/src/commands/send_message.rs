use crate::Config;

pub async fn send_message(config: Config, payload: Vec<u8>) -> anyhow::Result<()> {
    let private_key = config.owner_private_key;
    let contracts = super::init_contracts(config)?;
    let tx = contracts.submit_call(payload, &private_key).await?;
    println!("Sent message.\nTransaction hash: {tx:?}");
    Ok(())
}
