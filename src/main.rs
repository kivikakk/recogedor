use anyhow::{Context, Result};
use futures::join;
use std::fs;
use toml::Table;

mod endpoint;
mod imap;

use endpoint::Endpoint;

fn endpoints_from_config(config: &str) -> Result<(Endpoint, Endpoint)> {
    let table = config
        .parse::<Table>()
        .context("no se pudo analizar la config")?;
    Ok((
        Endpoint::from_config("src", table.get("src"))?,
        Endpoint::from_config("dest", table.get("dest"))?,
    ))
}

#[tokio::main]
async fn main() -> Result<()> {
    let (src, dest) = endpoints_from_config(
        &fs::read_to_string("config.toml").context("no se pudo leer config.toml")?,
    )?;

    println!("config leída con éxito");

    println!("comprobando src y dest ...");
    let s = src.connect_reader();
    let d = dest.connect_writer();
    let (sr, dr) = join!(s, d);

    let (mut sr, mut dr) = (sr?, dr?);

    sr.inbox().await?;

    loop {
        sr.idle().await?;

        for mail in sr.read().await? {
            if mail.flagged {
                println!("ya copiado, saltando ...");
            } else {
                dr.append(&mail).await?;
                sr.flag(mail.uid).await?;
            }
        }
    }
}
