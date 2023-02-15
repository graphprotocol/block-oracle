use anyhow::Context;
use glob::glob;
use std::{fs::write, path::PathBuf};
use xshell::{cmd, Shell};

const JSONNET_SAMPLES_DIRECTORY: &str = "crates/oracle/message-examples/jsonnet-examples";
const JSON_SAMPLES_DIRECTORY: &str = "crates/oracle/message-examples/";

fn compile() -> anyhow::Result<()> {
    let sh = Shell::new()?;
    for jsonnet_file in glob(&format!("{JSONNET_SAMPLES_DIRECTORY}/*.jsonnet"))? {
        let jsonnet_path = jsonnet_file?;
        let json = cmd!(sh, "jsonnet {jsonnet_path}")
            .read()
            .context("jsonnet failed")?;
        let target_file_name = {
            let mut base_path = PathBuf::from(JSON_SAMPLES_DIRECTORY);
            let jsonnet_file_name = jsonnet_path.file_name().unwrap().to_string_lossy();
            let json_file_name = jsonnet_file_name.trim_end_matches("net");
            base_path.push(json_file_name);
            base_path
        };
        write(target_file_name, json)?;
    }
    Ok(())
}

pub fn encode(calldata: bool) -> anyhow::Result<()> {
    let calldata = calldata.then_some("--calldata");
    compile()?;
    let sh = Shell::new()?;
    cmd!(sh, "cargo build --package block-oracle")
        .quiet()
        .run()?;
    for json_file in glob(&format!("{JSON_SAMPLES_DIRECTORY}/*.json"))? {
        let json_path = json_file?;
        let output = cmd!(
            sh,
            "./target/debug/block-oracle encode {calldata...} {json_path}"
        )
        .read()?;
        let file_name = json_path.to_string_lossy();
        let sample_name = file_name.trim_end_matches(".json");
        println!("[sample: {sample_name}]\n{output}\n");
    }
    Ok(())
}
