use anyhow::{Context, Result};
use clap::{arg, command, value_parser};
use futures::future::try_join_all;
use std::path::PathBuf;

mod config;
mod endpoint;
mod imap;

use config::Job;
use endpoint::IdleResult;

#[tokio::main]
async fn main() -> Result<()> {
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
    let jobs = config::from_file(&config_path)
        .with_context(|| format!("leyando {}", config_path.display()))?;
    println!("config leída con éxito");

    let mut futs = vec![];
    for (name, job) in &jobs {
        futs.push(run_job(name, job));
    }

    try_join_all(futs).await?;
    Ok(())
}

async fn prep_src(name: &str, job: &Job) -> Result<Box<dyn endpoint::EndpointReader>> {
    let mut src = job
        .src
        .connect_reader()
        .await
        .with_context(|| format!("connecting reader {}", name))?;

    src.inbox()
        .await
        .with_context(|| format!("inboxing {}", name))?;

    Ok(src)
}

async fn run_job(name: &str, job: &Job) -> Result<()> {
    let mut src = prep_src(name, job).await?;

    loop {
        let mut dest: Option<Box<dyn endpoint::EndpointWriter>> = None;

        for mail in src
            .read()
            .await
            .with_context(|| format!("reading {}", name))?
        {
            if mail.flagged {
                println!("[{}] ya copiado, saltando ...", name);
            } else {
                let d = match dest {
                    Some(ref mut d) => d,
                    None => dest.insert(
                        job.dest
                            .connect_writer()
                            .await
                            .with_context(|| format!("connecting writer {}", name))?,
                    ),
                };
                d.append(&mail)
                    .await
                    .with_context(|| format!("appending {}", name))?;
                src.flag(mail.uid)
                    .await
                    .with_context(|| format!("flagging {}", name))?;
            }
        }

        if let Some(mut d) = dest {
            d.disconnect()
                .await
                .with_context(|| format!("disconnecting {}", name))?;
        }

        'idle: loop {
            match src
                .idle()
                .await
                .with_context(|| format!("idling {}", name))?
            {
                IdleResult::Exists => break 'idle,
                IdleResult::ReIdle => continue 'idle,
                IdleResult::ReConnect => {
                    src = prep_src(name, job).await?;
                    break 'idle;
                }
            }
        }
    }
}
