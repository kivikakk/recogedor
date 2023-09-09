use crate::endpoint;
use anyhow::{Context, Result};
use async_imap::{
    extensions::idle::IdleResponse,
    imap_proto::types::{MailboxDatum, Response, Status},
};
use async_trait::async_trait;
use futures::TryStreamExt;
use std::time::Duration;
use tokio::net::TcpStream;

pub(crate) struct ImapEndpoint {
    name: String,
    host: String,
    ip: Option<String>,
    port: u16,
    user: String,
    pass: String,
}

impl ImapEndpoint {
    pub(crate) fn from_config(name: &str, value: &toml::Value) -> Result<ImapEndpoint> {
        let table = value
            .as_table()
            .with_context(|| format!("imap {} no es tabla", name))?;
        let host = table
            .get("host")
            .with_context(|| format!("falta el host imap {}", name))?
            .as_str()
            .with_context(|| format!("el host imap {} no es una cadena", name))?
            .to_string();
        let ip = match table.get("ip") {
            Some(v) => Some(
                v.as_str()
                    .with_context(|| format!("el ip imap {} no es una cadena", name))?
                    .to_string(),
            ),
            None => None,
        };
        let port = table
            .get("port")
            .with_context(|| format!("falta el pureto imap {}", name))?
            .as_integer()
            .with_context(|| format!("el pureto imap {} no es entero", name))?
            .try_into()
            .with_context(|| format!("el puero imap {} no está dentro del alcance", name))?;
        let user = table
            .get("user")
            .with_context(|| format!("falta el usuario imap {}", name))?
            .as_str()
            .with_context(|| format!("el usuario imap {} no es una cadena", name))?
            .to_string();
        let pass = table
            .get("pass")
            .with_context(|| format!("falta la contraseña imap {}", name))?
            .as_str()
            .with_context(|| format!("la contraseña imap {} no es una cadena", name))?
            .to_string();
        Ok(ImapEndpoint {
            name: name.to_string(),
            host,
            ip,
            port,
            user,
            pass,
        })
    }

    pub(crate) async fn connect(self) -> Result<ImapEndpointClient> {
        ImapEndpointClient::connect(self).await
    }
}

pub(crate) struct ImapEndpointClient {
    name: String,
    imap_session: Option<async_imap::Session<async_native_tls::TlsStream<TcpStream>>>,
}

impl ImapEndpointClient {
    async fn connect(ie: ImapEndpoint) -> Result<ImapEndpointClient> {
        println!("[{}] conectando tcp ...", ie.name);
        let addr = if let Some(ref ip) = ie.ip {
            (ip.as_ref(), ie.port)
        } else {
            (&*ie.host, ie.port)
        };
        let tcp_stream = TcpStream::connect(addr).await?;
        let tls = async_native_tls::TlsConnector::new();
        println!("[{}] conectando tls ...", ie.name);
        let tls_stream = tls.connect(&*ie.host, tcp_stream).await?;

        println!("[{}] conectando imap ...", ie.name);
        let client = async_imap::Client::new(tls_stream);
        println!("[{}] iniciando sesión ...", ie.name);
        let imap_session = Some(client.login(&*ie.user, &*ie.pass).await.map_err(|e| e.0)?);
        println!("[{}] (voz hacker) estoy dentro", ie.name);

        Ok(ImapEndpointClient {
            name: ie.name,
            imap_session,
        })
    }
}

#[async_trait]
impl endpoint::EndpointReader for ImapEndpointClient {
    async fn inbox(&mut self) -> Result<()> {
        let imap_session = self.imap_session.as_mut().context("sin sesión imap")?;
        println!("[{}] buscando correos nuevos de buzón INBOX ...", self.name);
        imap_session.select("INBOX").await?;
        Ok(())
    }

    async fn idle(&mut self) -> Result<endpoint::IdleResult> {
        println!("[{}] comenzando IDLE ...", self.name);
        let imap_session = self.imap_session.take().context("sin sesión imap")?;
        let mut idle = imap_session.idle();
        idle.init().await?;

        println!("[{}] comenzó.", self.name);
        let ir = 'idle: loop {
            let (idle_wait, _interrupt) = idle.wait_with_timeout(Duration::from_secs(10 * 60));
            println!("[{}] espera ...", self.name);

            match idle_wait.await? {
                IdleResponse::NewData(data) => match &data.parsed() {
                    Response::MailboxData(MailboxDatum::Exists(n)) => {
                        println!("[{}] tiene EXISTS: {}", self.name, n);
                        break 'idle endpoint::IdleResult::Exists;
                    }
                    Response::Data {
                        status: Status::Bye,
                        ..
                    } => {
                        println!("[{}] given Bye", self.name);
                        return Ok(endpoint::IdleResult::ReConnect);
                    }
                    parsed => {
                        println!("[{}] ignoring unknown: {:?}", self.name, parsed);
                    }
                },
                IdleResponse::Timeout => {
                    println!("[{}] got our timeout", self.name);
                    break 'idle endpoint::IdleResult::ReIdle;
                }
                other => {
                    println!("[{}] got other idle: {:?}", self.name, other);
                    return Ok(endpoint::IdleResult::ReConnect);
                }
            }
        };

        println!("[{}] despierto!", self.name);
        let imap_session = idle.done().await?;
        _ = self.imap_session.insert(imap_session);
        Ok(ir)
    }

    async fn read(&mut self) -> Result<Vec<endpoint::Message>> {
        let imap_session = self.imap_session.as_mut().context("sin sesión imap")?;
        println!("[{}] leyendo ...", self.name);
        let messages_stream = imap_session.fetch("1:*", "(UID FLAGS RFC822)").await?;
        let messages: Vec<_> = messages_stream.try_collect().await?;

        let mut result: Vec<endpoint::Message> = vec![];
        for message in &messages {
            result.push(message.try_into()?);
        }

        Ok(result)
    }

    async fn flag(&mut self, uid: u32) -> Result<()> {
        let imap_session = self.imap_session.as_mut().context("sin sesión imap")?;
        println!("[{}] marcado copiado ...", self.name);
        let updates_stream = imap_session
            .uid_store(format!("{}", uid), "+FLAGS (Recogido)")
            .await?;
        let _updates: Vec<_> = updates_stream.try_collect().await?;
        Ok(())
    }
}

#[async_trait]
impl endpoint::EndpointWriter for ImapEndpointClient {
    async fn append(&mut self, message: &endpoint::Message) -> Result<()> {
        let imap_session = self.imap_session.as_mut().context("sin sesión imap")?;
        println!("[{}] adjuntando mensaje ...", self.name);
        Ok(imap_session.append("INBOX", &message.body).await?)
    }
}
