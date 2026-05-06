//! Email and SMS provider traits and implementations

use async_trait::async_trait;
use lettre::AsyncTransport;

use crate::error::ApiError;

/// Email provider abstraction
#[async_trait]
pub trait EmailProvider: Send + Sync {
    /// Send an email
    async fn send_email(
        &self,
        to: &str,
        subject: &str,
        body_plain: &str,
        body_html: Option<&str>,
    ) -> Result<String, ApiError>;
}

/// SMS provider abstraction
#[async_trait]
pub trait SmsProvider: Send + Sync {
    /// Send an SMS
    async fn send_sms(&self, to: &str, message: &str) -> Result<String, ApiError>;
}

/// No-op email provider that logs instead of sending
pub struct NoopEmailProvider;

impl NoopEmailProvider {
    pub fn new() -> Self {
        Self
    }
}

impl Default for NoopEmailProvider {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl EmailProvider for NoopEmailProvider {
    async fn send_email(
        &self,
        to: &str,
        subject: &str,
        body_plain: &str,
        _body_html: Option<&str>,
    ) -> Result<String, ApiError> {
        tracing::info!(
            "NOOP Email to={} subject={} body={}",
            to,
            subject,
            body_plain
        );
        Ok("noop-message-id".to_string())
    }
}

/// No-op SMS provider that logs instead of sending
pub struct NoopSmsProvider;

impl NoopSmsProvider {
    pub fn new() -> Self {
        Self
    }
}

impl Default for NoopSmsProvider {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl SmsProvider for NoopSmsProvider {
    async fn send_sms(&self, to: &str, message: &str) -> Result<String, ApiError> {
        tracing::info!("NOOP SMS to={} message={}", to, message);
        Ok("noop-message-id".to_string())
    }
}

/// SMTP email provider using lettre
pub struct SmtpEmailProvider {
    transport: lettre::AsyncSmtpTransport<lettre::Tokio1Executor>,
    from_address: String,
}

impl SmtpEmailProvider {
    pub fn new(smtp_url: &str, from_address: &str) -> Result<Self, ApiError> {
        let transport = lettre::AsyncSmtpTransport::<lettre::Tokio1Executor>::from_url(smtp_url)
            .map_err(|e| ApiError::Internal(format!("Failed to create SMTP transport: {}", e)))?
            .build();

        Ok(Self {
            transport,
            from_address: from_address.to_string(),
        })
    }
}

#[async_trait]
impl EmailProvider for SmtpEmailProvider {
    async fn send_email(
        &self,
        to: &str,
        subject: &str,
        body_plain: &str,
        body_html: Option<&str>,
    ) -> Result<String, ApiError> {
        let from = lettre::message::Mailbox::new(
            None,
            self.from_address
                .parse()
                .map_err(|e| ApiError::Internal(format!("Invalid from address: {}", e)))?,
        );

        let to = lettre::message::Mailbox::new(
            None,
            to.parse()
                .map_err(|e| ApiError::BadRequest(format!("Invalid recipient: {}", e)))?,
        );

        let builder = lettre::Message::builder()
            .from(from)
            .to(to)
            .subject(subject);

        let body = if let Some(html) = body_html {
            lettre::message::MultiPart::alternative()
                .singlepart(
                    lettre::message::SinglePart::builder()
                        .header(lettre::message::header::ContentType::TEXT_PLAIN)
                        .body(body_plain.to_string()),
                )
                .singlepart(
                    lettre::message::SinglePart::builder()
                        .header(lettre::message::header::ContentType::TEXT_HTML)
                        .body(html.to_string()),
                )
        } else {
            lettre::message::MultiPart::alternative().singlepart(
                lettre::message::SinglePart::builder()
                    .header(lettre::message::header::ContentType::TEXT_PLAIN)
                    .body(body_plain.to_string()),
            )
        };

        let message = builder
            .multipart(body)
            .map_err(|e| ApiError::Internal(format!("Failed to build email: {}", e)))?;

        let response = self
            .transport
            .send(message)
            .await
            .map_err(|e| ApiError::Internal(format!("SMTP send failed: {}", e)))?;

        Ok(response
            .first_line()
            .map(|s| s.to_string())
            .unwrap_or_else(|| "sent".to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_noop_email_provider() {
        let provider = NoopEmailProvider::new();
        let id = provider
            .send_email("to@example.com", "Subject", "Body", None)
            .await
            .unwrap();
        assert_eq!(id, "noop-message-id");
    }

    #[tokio::test]
    async fn test_noop_sms_provider() {
        let provider = NoopSmsProvider::new();
        let id = provider.send_sms("+905551234567", "Hello").await.unwrap();
        assert_eq!(id, "noop-message-id");
    }
}
