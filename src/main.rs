use anyhow::{Context, Result};
use futures::future::try_join_all;

mod config;
mod endpoint;
mod imap;

use config::Job;

#[tokio::main]
async fn main() -> Result<()> {
    let jobs = config::from_file("config.toml").context("leyando config.toml")?;
    println!("config leída con éxito");

    let mut futs = vec![];
    for (name, job) in &jobs {
        futs.push(run_job(name, job));
    }

    try_join_all(futs).await?;
    Ok(())
}

async fn run_job(name: &str, job: &Job) -> Result<()> {
    let mut src = job
        .src
        .connect_reader()
        .await
        .with_context(|| format!("connecting reader {}", name))?;

    src.inbox()
        .await
        .with_context(|| format!("inboxing {}", name))?;

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

        src.idle()
            .await
            .with_context(|| format!("idling {}", name))?;
    }
}
