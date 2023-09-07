use crate::endpoint;
use anyhow::{Context, Result};
use async_trait::async_trait;
use tokio::net::TcpStream;

pub(crate) struct ImapEndpoint {
    host: String,
    port: u16,
    user: String,
    pass: String,
}

impl ImapEndpoint {
    pub(crate) fn from_config(value: &toml::Value) -> Result<ImapEndpoint> {
        let table = value.as_table().context("imap no es tabla")?;
        let host = table
            .get("host")
            .context("falta el host imap")?
            .as_str()
            .context("el host imap no es una cadena")?
            .to_string();
        let port = table
            .get("port")
            .context("falta el pureto imap")?
            .as_integer()
            .context("el pureto imap no es entero")?
            .try_into()
            .context("el puero imap no está dentro del alcance")?;
        let user = table
            .get("user")
            .context("falta el usuario imap")?
            .as_str()
            .context("el usuario imap no es una cadena")?
            .to_string();
        let pass = table
            .get("pass")
            .context("falta la contraseña imap")?
            .as_str()
            .context("la contraseña imap no es una cadena")?
            .to_string();
        Ok(ImapEndpoint {
            host,
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
    imap_session: async_imap::Session<async_native_tls::TlsStream<TcpStream>>,
}

impl ImapEndpointClient {
    async fn connect(ie: ImapEndpoint) -> Result<ImapEndpointClient> {
        println!("[imap] conectando tcp ...");
        let tcp_stream = TcpStream::connect((&*ie.host, ie.port)).await?;
        let tls = async_native_tls::TlsConnector::new();
        println!("[imap] conectando tls ...");
        let tls_stream = tls.connect(&*ie.host, tcp_stream).await?;

        println!("[imap] conectando imap ...");
        let client = async_imap::Client::new(tls_stream);
        println!("[imap] iniciando sesión ...");
        let imap_session = client.login(&*ie.user, &*ie.pass).await.map_err(|e| e.0)?;
        println!("[imap] (voz hacker) estoy dentro");

        Ok(ImapEndpointClient { imap_session })
    }
}

#[async_trait]
impl endpoint::EndpointWriter for ImapEndpointClient {
    async fn append(&mut self, content: &[u8]) -> Result<()> {
        println!("[imap] adjuntando mensaje ...");
        Ok(self.imap_session.append("INBOX", content).await?)
    }
}
