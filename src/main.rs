use anyhow::{Context, Result};
use clap::{arg, command, value_parser};
use log::{debug, info};
use std::path::PathBuf;

mod config;
mod endpoint;
mod imap;
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
        .get_matches();
    let config_path = matches
        .get_one::<PathBuf>("config")
        .cloned()
        .unwrap_or("config.toml".into());
    let config = config::from_file(&config_path)
        .with_context(|| format!("reading {}", config_path.display()))?;
    info!("config read OK");
    debug!("{}", config.script);

    run(&config).await?;
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
        let mut closure = config.script.closure(&config.dests);

        for mail in src.read().await.context("reading")? {
            let actions = closure.process(&mail)?;
            for action in actions {
                closure.action(&mail, action, &mut src).await?;
            }
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
