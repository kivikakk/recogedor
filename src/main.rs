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
    println!("[{}] comprobando src ...", name);

    let mut src = job.src.connect_reader().await?;

    src.inbox().await?;

    loop {
        let mut dest: Option<Box<dyn endpoint::EndpointWriter>> = None;

        for mail in src.read().await? {
            if mail.flagged {
                println!("ya copiado, saltando ...");
            } else {
                let d = match dest {
                    Some(ref mut d) => d,
                    None => dest.insert(job.dest.connect_writer().await?),
                };
                d.append(&mail).await?;
                src.flag(mail.uid).await?;
            }
        }

        if let Some(mut d) = dest {
            d.disconnect().await?;
        }

        src.idle().await?;
    }
}
