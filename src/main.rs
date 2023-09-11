use anyhow::{Context, Result};
use std::fs;
use toml::Table;

mod endpoint;
mod imap;

use endpoint::{Endpoint, EndpointWriter};

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
    let (cfg_src, cfg_dest) = endpoints_from_config(
        &fs::read_to_string("config.toml").context("no se pudo leer config.toml")?,
    )?;

    println!("config leída con éxito");

    println!("comprobando src ...");

    let mut src = cfg_src.connect_reader().await?;

    src.inbox().await?;

    loop {
        let mut dest: Option<Box<dyn EndpointWriter>> = None;

        for mail in src.read().await? {
            if mail.flagged {
                println!("ya copiado, saltando ...");
            } else {
                let d = match dest {
                    Some(ref mut d) => d,
                    None => dest.insert(cfg_dest.connect_writer().await?),
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
