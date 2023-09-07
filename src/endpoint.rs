use anyhow::{bail, Context, Result};
use async_trait::async_trait;

use crate::imap::ImapEndpoint;
use crate::jmap::JmapEndpoint;

pub(crate) enum Endpoint {
    Imap(ImapEndpoint),
    Jmap(JmapEndpoint),
}

impl Endpoint {
    pub(crate) fn from_config(which: &str, value: Option<&toml::Value>) -> Result<Self> {
        let table = value
            .with_context(|| format!("falta {} ", which))?
            .as_table()
            .with_context(|| format!("{} no es tabla", which))?;
        let ep = match (table.get("imap"), table.get("jmap")) {
            (None, None) => bail!("se esperaba imap o jmap para {}, ninguno dadon", which),
            (Some(_), Some(_)) => bail!("se esperaba imap o jmap para {}, ambos dados", which),
            (Some(im), None) => Endpoint::Imap(ImapEndpoint::from_config(im)?),
            (None, Some(jm)) => Endpoint::Jmap(JmapEndpoint::from_config(jm)?),
        };
        Ok(ep)
    }

    pub(crate) async fn connect_reader(self) -> Result<Box<dyn EndpointReader>> {
        match self {
            Endpoint::Imap(_) => bail!("aún no hay lector IMAP"),
            Endpoint::Jmap(je) => {
                let jec = je.connect().await?;
                Ok(Box::new(jec))
            }
        }
    }
    pub(crate) async fn connect_writer(self) -> Result<Box<dyn EndpointWriter>> {
        match self {
            Endpoint::Imap(ie) => {
                let iec = ie.connect().await?;
                Ok(Box::new(iec))
            }
            Endpoint::Jmap(_) => bail!("aún no hay escritor JMAP"),
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
