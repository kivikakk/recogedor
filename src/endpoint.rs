use anyhow::{bail, Context, Error, Result};
use async_imap::types::Flag;
use async_trait::async_trait;
use std::collections::HashSet;

use crate::{ast::RecipientPattern, imap::ImapEndpoint};

#[derive(Clone)]
pub(crate) enum Endpoint {
    Imap(ImapEndpoint),
}

impl Endpoint {
    pub(crate) fn from_config(which: &str, value: &toml::Value) -> Result<Self> {
        let table = value
            .as_table()
            .with_context(|| format!("{} not a table", which))?;
        let tipo = table
            .get("type")
            .with_context(|| format!("{} config missing type", which))?
            .as_str()
            .with_context(|| format!("{} config type not string", which))?;
        if tipo != "imap" {
            bail!("unknown type {}", tipo);
        }
        Ok(Endpoint::Imap(ImapEndpoint::from_config(which, table)?))
    }

    pub(crate) async fn connect_source(&self) -> Result<Box<dyn SourceEndpoint>> {
        match self {
            Endpoint::Imap(ie) => {
                let iec = ie.connect().await?;
                Ok(Box::new(iec))
            }
        }
    }
    pub(crate) async fn connect_destination(&self) -> Result<Box<dyn DestinationEndpoint>> {
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
    pub(crate) body: Vec<u8>,
    flags: HashSet<String>,
    recipients: HashSet<Recipient>,
}

impl Message {
    pub(crate) fn flagged(&self, flag: &str) -> bool {
        self.flags.contains(flag)
    }

    pub(crate) fn received_by(&self, pattern: &RecipientPattern) -> bool {
        self.recipients.iter().any(|r| pattern.matches(r))
    }
}

impl std::convert::TryFrom<&async_imap::types::Fetch> for Message {
    type Error = Error;

    fn try_from(message: &async_imap::types::Fetch) -> Result<Self> {
        let body = message.body().context("message body missing")?.to_vec();

        let flags = message
            .flags()
            .map(|f| match f {
                Flag::Custom(f) => f.to_string(),
                f => format!("\\{:?}", f).to_string(),
            })
            .collect();

        let envelope = message.envelope().context("message envelope missing")?;
        let mut recipients = HashSet::new();
        for list in [&envelope.to, &envelope.cc, &envelope.bcc]
            .into_iter()
            .flatten()
        {
            for addr in list {
                recipients.insert(Recipient {
                    mailbox: addr
                        .mailbox
                        .as_ref()
                        .and_then(|at| Some(at.to_vec()))
                        .unwrap_or(vec![]),
                    host: addr
                        .host
                        .as_ref()
                        .and_then(|at| Some(at.to_vec()))
                        .unwrap_or(vec![]),
                });
            }
        }
        Ok(Message {
            uid: message.uid.context("message uid missing")?,
            body,
            flags,
            recipients,
        })
    }
}

#[derive(PartialEq, Eq, Hash)]
pub(crate) struct Recipient {
    pub(crate) mailbox: Vec<u8>,
    pub(crate) host: Vec<u8>,
}

pub(crate) enum IdleResult {
    Exists,
    ReIdle,
    ReConnect,
}

#[async_trait]
pub(crate) trait EndpointSelector {
    async fn select(&mut self, folder: &str) -> Result<()>;
}

#[async_trait]
pub(crate) trait EndpointReader {
    async fn idle(&mut self) -> Result<IdleResult>;
    async fn read(&mut self) -> Result<Vec<Message>>;
}

#[async_trait]
pub(crate) trait EndpointFlagger {
    async fn flag(&mut self, uid: u32, flag: &str) -> Result<()>;
    async fn delete(&mut self, uid: u32) -> Result<()>;
    async fn expunge(&mut self) -> Result<()>;
}

#[async_trait]
pub(crate) trait SourceEndpoint:
    EndpointSelector + EndpointReader + EndpointFlagger
{
}
impl<T: EndpointSelector + EndpointReader + EndpointFlagger> SourceEndpoint for T {}

#[async_trait]
pub(crate) trait EndpointWriter {
    async fn append(&mut self, folder: &str, message: &Message) -> Result<()>;
    async fn disconnect(&mut self) -> Result<()>;
}

#[async_trait]
pub(crate) trait DestinationEndpoint: EndpointSelector + EndpointWriter {}
impl<T: EndpointSelector + EndpointWriter> DestinationEndpoint for T {}
