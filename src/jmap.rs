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
        JmapEndpointClient::connect(self).await
    }
}

pub(crate) struct JmapEndpointClient {
    client: Client,
    inbox_id: String,
}

impl JmapEndpointClient {
    pub(crate) async fn connect(je: JmapEndpoint) -> Result<JmapEndpointClient> {
        println!("[jmap] conectando ...");
        let client = Client::new()
            .credentials(&*je.bearer)
            .connect(&je.url)
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

        Ok(JmapEndpointClient { client, inbox_id })
    }
}

#[async_trait]
impl endpoint::EndpointReader for JmapEndpointClient {
    async fn first(&mut self) -> Result<Option<Vec<u8>>> {
        println!(
            "[jmap] buscando correos nuevos de buzón con id {:?} ...",
            self.inbox_id
        );

        let email_ids = self
            .client
            .email_query(
                email::query::Filter::in_mailbox(&self.inbox_id).into(),
                [email::query::Comparator::received_at().ascending()].into(),
            )
            .await?
            .take_ids();

        println!("[jmap] encontré {} nuevo(s) correo(s).", email_ids.len());

        for email_id in &email_ids {
            let email = self
                .client
                .email_get(
                    email_id,
                    [
                        email::Property::TextBody,
                        email::Property::HtmlBody,
                        email::Property::BodyValues,
                        email::Property::Header(email::Header.parse("header::all")
                    ]
                    .into(),
                )
                .await?
                .context("mensaje no encontrado")?;
            println!("email: {:?}", email);

            dump_email(0, email.body_structure().context("missing structure")?);
        }

        Ok(Some("".into()))
    }
}

fn dump_email(level: u8, part: &email::EmailBodyPart) {
    println!("{} {:?}", level, part);
    if let Some(sub_parts) = part.sub_parts() {
        for sub_part in sub_parts {
            dump_email(level + 1, sub_part);
        }
    }
}
