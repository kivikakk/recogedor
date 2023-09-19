use std::collections::HashSet;

use anyhow::{bail, Context, Error, Result};
use async_imap::types::Flag;
use async_trait::async_trait;

use crate::{imap::ImapEndpoint, script::Pattern};

#[derive(Clone)]
pub(crate) enum Endpoint {
    Imap(ImapEndpoint),
}

impl Endpoint {
    pub(crate) fn from_config(which: &str, value: &toml::Value) -> Result<Self> {
        let table = value
            .as_table()
            .with_context(|| format!("{} no es tabla", which))?;
        let tipo = table
            .get("type")
            .with_context(|| format!("la config para {} no hay tipo", which))?
            .as_str()
            .with_context(|| format!("el tipo de la config para {} no es una cadena", which))?;
        if tipo != "imap" {
            bail!("no sé este tipo {}", tipo);
        }
        Ok(Endpoint::Imap(ImapEndpoint::from_config(which, table)?))
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
    pub(crate) body: Vec<u8>,
    flags: HashSet<String>,
    recipients: HashSet<Vec<u8>>,
}

impl Message {
    pub(crate) fn flagged(&self, flag: &str) -> bool {
        self.flags.contains(flag)
    }

    pub(crate) fn received_by(&self, pattern: &Pattern) -> bool {
        // Patterns have this syntax:
        // (?<mailbox>[^+@]+)?(?:\+(?<plus>[^@]+))?@(?<host>.+)?
        // Note that '@' is a part of every pattern.

        // XXX
        false
    }
}

impl std::convert::TryFrom<&async_imap::types::Fetch> for Message {
    type Error = Error;

    fn try_from(message: &async_imap::types::Fetch) -> Result<Self> {
        let body = message
            .body()
            .context("falta el cuerpo del mensaje")?
            .to_vec();

        let flags = message
            .flags()
            .map(|f| match f {
                Flag::Custom(f) => f.to_string(),
                f => format!("\\{:?}", f).to_string(),
            })
            .collect();

        let envelope = message.envelope().context("mensaje no tiene sobres")?;
        let mut recipients = HashSet::new();
        for list in [&envelope.to, &envelope.cc, &envelope.bcc] {
            if let Some(list) = list {
                for addr in list {
                    let mut recipient = Vec::<u8>::with_capacity(
                        addr.mailbox.as_ref().map_or(0, |v| v.len())
                            + 1
                            + addr.host.as_ref().map_or(0, |v| v.len()),
                    );
                    if let Some(mailbox) = &addr.mailbox {
                        recipient.extend_from_slice(&mailbox);
                    }
                    recipient.push(b'@');
                    if let Some(host) = &addr.host {
                        recipient.extend_from_slice(&host);
                    }
                    recipients.insert(recipient);
                }
            }
        }
        Ok(Message {
            uid: message.uid.context("mensaje no tiene uid")?,
            body,
            flags,
            recipients,
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
