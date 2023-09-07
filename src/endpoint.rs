use anyhow::{bail, Context, Result};
use async_trait::async_trait;

use crate::imap::ImapEndpoint;

pub(crate) enum Endpoint {
    Imap(ImapEndpoint),
}

impl Endpoint {
    pub(crate) fn from_config(which: &str, value: Option<&toml::Value>) -> Result<Self> {
        let table = value
            .with_context(|| format!("falta {} ", which))?
            .as_table()
            .with_context(|| format!("{} no es tabla", which))?;
        let ep = match (table.get("imap"), ) {
            (None,) => bail!("se esperaba imap para {}, ninguno dadon", which),
            (Some(im), ) => Endpoint::Imap(ImapEndpoint::from_config(im)?),
        };
        Ok(ep)
    }

    pub(crate) async fn connect_reader(self) -> Result<Box<dyn EndpointReader>> {
        match self {
            Endpoint::Imap(ie) => {
                let iec = ie.connect().await?;
                Ok(Box::new(iec))
            }
        }
    }
    pub(crate) async fn connect_writer(self) -> Result<Box<dyn EndpointWriter>> {
        match self {
            Endpoint::Imap(ie) => {
                let iec = ie.connect().await?;
                Ok(Box::new(iec))
            }
        }
    }
}

#[async_trait]
pub(crate) trait EndpointReader {
    async fn first(&mut self) -> Result<Option<Vec<u8>>>;
}
#[async_trait]
pub(crate) trait EndpointWriter {
    async fn append(&mut self, content: &[u8]) -> Result<()>;
}
