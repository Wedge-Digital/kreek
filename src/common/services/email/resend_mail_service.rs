use super::email_service::{EmailError, IEmailService};
use async_trait::async_trait;
use reqwest::Client;
use serde::Serialize;

pub struct ResendMailService {
    client: Client,
    api_url: String,
    api_key: String,
    sender_email: String,
    sender_name: String,
}

impl ResendMailService {
    pub fn new(api_key: String, sender_email: String, sender_name: String) -> Self {
        ResendMailService {
            client: Client::new(),
            api_url: "https://api.resend.com/emails".into(),
            api_key,
            sender_email,
            sender_name,
        }
    }

    #[cfg(test)]
    fn with_base_url(
        base_url: String,
        api_key: String,
        sender_email: String,
        sender_name: String,
    ) -> Self {
        ResendMailService {
            client: Client::new(),
            api_url: format!("{}/emails", base_url),
            api_key,
            sender_email,
            sender_name,
        }
    }
}

#[derive(Serialize)]
struct ResendRequest {
    from: String,
    to: Vec<String>,
    subject: String,
    html: String,
}

#[async_trait]
impl IEmailService for ResendMailService {
    async fn send(&self, to: Vec<String>, subject: String, html: String) -> Result<(), EmailError> {
        let payload = ResendRequest {
            from: format!("{} <{}>", self.sender_name, self.sender_email),
            to,
            subject,
            html,
        };

        let response = self
            .client
            .post(&self.api_url)
            .bearer_auth(&self.api_key)
            .json(&payload)
            .send()
            .await
            .map_err(|e| EmailError::Network(e.to_string()))?;

        if response.status().is_success() {
            return Ok(());
        }

        let status = response.status().as_u16();
        let body = response.text().await.unwrap_or_default();
        Err(EmailError::ApiRejected { status, body })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mockito::{Matcher, Server};

    fn service(base_url: String) -> ResendMailService {
        ResendMailService::with_base_url(
            base_url,
            "test-api-key".into(),
            "sender@kreek.app".into(),
            "Kreek".into(),
        )
    }

    #[tokio::test]
    async fn send_success_renvoie_ok() {
        let mut server = Server::new_async().await;
        server
            .mock("POST", "/emails")
            .with_status(200)
            .with_body(r#"{"id":"msg-001"}"#)
            .create_async()
            .await;

        let result = service(server.url())
            .send(
                vec!["dest@example.com".into()],
                "Sujet".into(),
                "<p>Test</p>".into(),
            )
            .await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn send_api_rejected_renvoie_erreur_avec_status() {
        let mut server = Server::new_async().await;
        server
            .mock("POST", "/emails")
            .with_status(422)
            .with_body(r#"{"message":"Invalid email"}"#)
            .create_async()
            .await;

        let result = service(server.url())
            .send(
                vec!["dest@example.com".into()],
                "Sujet".into(),
                "<p>Test</p>".into(),
            )
            .await;

        assert!(matches!(
            result,
            Err(EmailError::ApiRejected { status: 422, .. })
        ));
    }

    #[tokio::test]
    async fn send_erreur_reseau_si_serveur_inaccessible() {
        let result = service("http://127.0.0.1:1".into())
            .send(
                vec!["dest@example.com".into()],
                "Sujet".into(),
                "<p>Test</p>".into(),
            )
            .await;

        assert!(matches!(result, Err(EmailError::Network(_))));
    }

    #[tokio::test]
    async fn send_transmet_le_bon_header_auth() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/emails")
            .match_header("authorization", "Bearer test-api-key")
            .with_status(200)
            .with_body(r#"{"id":"msg-001"}"#)
            .create_async()
            .await;

        service(server.url())
            .send(
                vec!["dest@example.com".into()],
                "Sujet".into(),
                "<p>Test</p>".into(),
            )
            .await
            .unwrap();

        mock.assert_async().await;
    }

    #[tokio::test]
    #[ignore = "envoie un vrai email — nécessite RESEND_API_KEY et RESEND_TEST_TO dans l'environnement"]
    async fn integration_envoi_email_reel() {
        let api_key =
            std::env::var("RESEND_API_KEY").expect("RESEND_API_KEY doit être défini pour ce test");
        let to =
            std::env::var("RESEND_TEST_TO").expect("RESEND_TEST_TO doit être défini pour ce test");

        let service = ResendMailService::new(api_key, "mailer@example.com".into(), "Kreek".into());

        let result = service.send(
            vec![to],
            "Test d'intégration Kreek".into(),
            "<h1>Test client mail</h1><p>Si tu vois cet email, le client Resend fonctionne.</p>".into(),
        ).await;

        assert!(result.is_ok(), "Erreur d'envoi : {:?}", result.err());
    }

    #[tokio::test]
    async fn send_transmet_le_bon_payload() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/emails")
            .match_body(Matcher::PartialJson(serde_json::json!({
                "from":    "Kreek <sender@kreek.app>",
                "to":      ["dest@example.com"],
                "subject": "Réinitialisation",
                "html":    "<p>Lien</p>"
            })))
            .with_status(200)
            .with_body(r#"{"id":"msg-001"}"#)
            .create_async()
            .await;

        service(server.url())
            .send(
                vec!["dest@example.com".into()],
                "Réinitialisation".into(),
                "<p>Lien</p>".into(),
            )
            .await
            .unwrap();

        mock.assert_async().await;
    }
}
