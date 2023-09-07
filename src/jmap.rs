use crate::endpoint;
use anyhow::{Context, Result};
use async_trait::async_trait;
use jmap_client::{client::Client, email, mailbox};

pub(crate) struct JmapEndpoint {
    url: String,
    bearer: String,
}

impl JmapEndpoint {
    pub(crate) fn from_config(value: &toml::Value) -> Result<JmapEndpoint> {
        let table = value.as_table().context("jmap no es tabla")?;
        let url = table
            .get("url")
            .context("falta la url jmap")?
            .as_str()
            .context("la url jmap no es una cadena")?
            .to_string();
        let bearer = table
            .get("bearer")
            .context("falta la portadora jmap")?
            .as_str()
            .context("la portadora jmap no es una cadena")?
            .to_string();
        Ok(JmapEndpoint { url, bearer })
    }

    pub(crate) async fn connect(self) -> Result<JmapEndpointClient> {
        let mut jec = JmapEndpointClient::new(self);
        jec.connect().await?;
        Ok(jec)
    }
}

pub(crate) struct JmapEndpointClient {
    je: JmapEndpoint,
}

impl JmapEndpointClient {
    fn new(je: JmapEndpoint) -> JmapEndpointClient {
        JmapEndpointClient { je }
    }

    async fn connect(&mut self) -> Result<()> {
        println!("[jmap] conectando ...");
        let client = Client::new()
            .credentials(&*self.je.bearer)
            .connect(&self.je.url)
            .await?;

        println!("[jmap] buscando INBOX ...");
        let inbox_id = client
            .mailbox_query(
                mailbox::query::Filter::role(mailbox::Role::Inbox).into(),
                None::<Vec<_>>,
            )
            .await?
            .take_ids()
            .pop()
            .context("xyz")?;

        // let mailbox = client.mailbox_get(inbox_id, None::<Vec<_>>).await?;
        // println!("[jmap] {:?}", mailbox);

        println!(
            "[jmap] buscando correos nuevos de buzón con id {:?} ...",
            inbox_id
        );

        let email_ids = client
            .email_query(
                email::query::Filter::in_mailbox(&inbox_id).into(),
                [email::query::Comparator::received_at().ascending()].into(),
            )
            .await?
            .take_ids();

        println!("[jmap] encontré {} nuevo(s) correo(s).", email_ids.len());

        Ok(())
    }
}

#[async_trait]
impl endpoint::EndpointReader for JmapEndpointClient {
    async fn first(&mut self) -> Result<Vec<u8>> {
        Ok(concat!(r#"<?xml version="1.0" encoding="UTF-8"?>"#,
            r#"<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">"#,
            r#"<plist version="1.0">"#,
            r#"<dict>"#,
            r#"	<key>conversation-id</key>"#,
            r#"	<integer>37045</integer>"#,
            r#"	<key>date-last-viewed</key>"#,
            r#"	<integer>1694069867</integer>"#,
            r#"	<key>date-received</key>"#,
            r#"	<integer>1594069740</integer>"#,
            r#"	<key>flags</key>"#,
            r#"	<integer>2983488513</integer>"#,
            r#"	<key>remote-id</key>"#,
            r#"	<string>11</string>"#,
            r#"</dict>"#,
            r#"</plist>"#).into())
    }
}
