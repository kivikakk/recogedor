use anyhow::{Context, Result};
use clap::{arg, command, value_parser};
use log::{debug, info};
use std::collections::HashMap;
use std::path::PathBuf;

mod config;
mod endpoint;
mod imap;
mod script;

use config::Config;
use endpoint::{Endpoint, EndpointReader, EndpointWriter, IdleResult};

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
        .with_context(|| format!("leyando {}", config_path.display()))?;
    info!("config leída con éxito");
    debug!("{}", config.script);

    run(&config).await?;
    Ok(())
}

async fn prep_src(endpoint: &Endpoint) -> Result<Box<dyn EndpointReader>> {
    let mut src = endpoint
        .connect_reader()
        .await
        .context("conectando lectora")?;

    src.inbox().await.context("seleccionando inbox")?;

    Ok(src)
}

async fn run(config: &Config) -> Result<()> {
    let mut src = prep_src(&config.src).await?;

    loop {
        let mut closure = config.script.closure(&config.dests);

        for mail in src.read().await.context("leyendo")? {
            let actions = closure.process(mail)?;
            for action in &actions {}
            // if mail.flagged {
            //     trace!("ya copiado, saltando ...");
            // } else {
            //     let d = match dest {
            //         Some(ref mut d) => d,
            //         None => dest.insert(
            //             job.dest
            //                 .connect_writer()
            //                 .await
            //                 .context("conectando escritora")?,
            //         ),
            //     };
            //     d.append(&mail).await.context("adjuntando")?;
            //     src.flag(mail.uid).await.context("marcando")?;
            // }
        }

        closure.finish().await?;

        'idle: loop {
            match src.idle().await.context("esperando")? {
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
