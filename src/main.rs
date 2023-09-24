use anyhow::{Context, Result};
use clap::{arg, command, value_parser};
use log::{debug, info};
use std::path::PathBuf;

mod ast;
mod config;
mod endpoint;
mod imap;
mod ir;
mod script;

use config::Config;
use endpoint::{Endpoint, EndpointReader, IdleResult};

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();

    let matches = command!()
        .arg(
            arg!(-c --config <FILE> "Path to config.toml")
                .required(false)
                .value_parser(value_parser!(PathBuf)),
        )
        .arg(arg!(-n --"dry-run" "Check that the config parses, but don't run."))
        .get_matches();
    let config_path = matches
        .get_one::<PathBuf>("config")
        .cloned()
        .unwrap_or("config.toml".into());
    let config = config::from_file(&config_path)
        .with_context(|| format!("reading {}", config_path.display()))?;
    info!("config read OK");
    debug!("{}", config.ir);

    if !*matches.get_one::<bool>("dry-run").unwrap_or(&false) {
        run(&config).await?;
    }
    Ok(())
}

async fn prep_src(endpoint: &Endpoint) -> Result<Box<dyn EndpointReader>> {
    let mut src = endpoint
        .connect_reader()
        .await
        .context("connecting reader")?;

    src.inbox().await.context("selecting inbox")?;

    Ok(src)
}

async fn run(config: &Config) -> Result<()> {
    let mut src = prep_src(&config.src).await?;

    loop {
        let mut closure = config.ir.closure();

        for mail in src.read().await.context("reading")? {
            closure.process(&mail, &mut src).await?;
        }

        closure.finish().await?;

        'idle: loop {
            match src.idle().await.context("IDLEing")? {
                IdleResult::Exists => break 'idle,
                IdleResult::ReIdle => continue 'idle,
                IdleResult::ReConnect => {
                    src = prep_src(&config.src).await?;
                    break 'idle;
                }
            }
        }
    }
}
