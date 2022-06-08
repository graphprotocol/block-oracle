use anyhow::Context;
use glob::glob;
use std::fs::write;
use xshell::{cmd, Shell};

const SAMPLES_DIRECTORY: &'static str = "crates/oracle-encoder/examples";

fn compile() -> anyhow::Result<()> {
    let sh = Shell::new()?;

    for jsonnet_file in glob(&format!("{}/*.jsonnet", SAMPLES_DIRECTORY))? {
        let mut jsonnet_path = jsonnet_file?;
        let json = cmd!(sh, "jsonnet {jsonnet_path}")
            .read()
            .context("jsonnet failed")?;
        let target_file_name = {
            jsonnet_path.set_extension("json");
            jsonnet_path
        };
        write(target_file_name, json)?;
    }
    Ok(())
}

pub fn encode() -> anyhow::Result<()> {
    compile()?;
    let sh = Shell::new()?;
    cmd!(sh, "cargo build --package oracle-encoder")
        .quiet()
        .run()?;
    for json_file in glob(&format!("{}/*.json", SAMPLES_DIRECTORY))? {
        let json_path = json_file?;
        let output = cmd!(sh, "./target/debug/oracle-encoder --json-path {json_path}").read()?;
        let file_name = json_path.to_string_lossy();
        let sample_name = file_name.trim_end_matches(".json");
        println!("[sample: {}]\n{}\n", sample_name, output);
    }
    Ok(())
}
