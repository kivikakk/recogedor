use anyhow::{bail, Context, Error, Result};
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
        let ep = match (table.get("imap"),) {
            (None,) => bail!("se esperaba imap para {}, ninguno dadon", which),
            (Some(im),) => Endpoint::Imap(ImapEndpoint::from_config(which, im)?),
        };
        Ok(ep)
    }

    pub(crate) async fn connect_reader(&self) -> Result<Box<dyn EndpointReader>> {
        match self {
            Endpoint::Imap(ie) => {
                let iec = ie.connect().await?;
                Ok(Box::new(iec))
            }
        }
    }
    pub(crate) async fn connect_writer(&self) -> Result<Box<dyn EndpointWriter>> {
        match self {
            Endpoint::Imap(ie) => {
                let iec = ie.connect().await?;
                Ok(Box::new(iec))
            }
        }
    }
}

pub(crate) struct Message {
    pub(crate) uid: u32,
    pub(crate) flagged: bool,
    pub(crate) body: Vec<u8>,
}

impl std::convert::TryFrom<&async_imap::types::Fetch> for Message {
    type Error = Error;

    fn try_from(message: &async_imap::types::Fetch) -> Result<Self> {
        let flagged = message.flags().any(|f| f == "Recogido".into());
        Ok(Message {
            uid: message.uid.context("mensaje no tiene uid")?,
            flagged,
            body: message
                .body()
                .context("falta el cuerpo del mensaje")?
                .to_vec(),
        })
    }
}

pub(crate) enum IdleResult {
    Exists,
    ReIdle,
    ReConnect,
}

#[async_trait]
pub(crate) trait EndpointReader {
    async fn inbox(&mut self) -> Result<()>;
    async fn idle(&mut self) -> Result<IdleResult>;
    async fn read(&mut self) -> Result<Vec<Message>>;
    async fn flag(&mut self, uid: u32) -> Result<()>;
}
#[async_trait]
pub(crate) trait EndpointWriter {
    async fn append(&mut self, message: &Message) -> Result<()>;
    async fn disconnect(&mut self) -> Result<()>;
}
