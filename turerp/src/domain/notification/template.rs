//! Handlebars-based template engine for notifications

use handlebars::Handlebars;
use serde_json::Value;

use crate::error::ApiError;

/// Template engine using Handlebars
#[derive(Clone)]
pub struct TemplateEngine {
    registry: Handlebars<'static>,
}

impl TemplateEngine {
    /// Create a new template engine with built-in defaults
    pub fn new() -> Self {
        let mut registry = Handlebars::new();
        Self::register_defaults(&mut registry);
        Self { registry }
    }

    fn register_defaults(registry: &mut Handlebars<'static>) {
        let defaults = default_templates();
        for (key, subject, body, html) in defaults {
            registry
                .register_template_string(&format!("{}_subject", key), subject)
                .ok();
            registry
                .register_template_string(&format!("{}_body", key), body)
                .ok();
            if let Some(h) = html {
                registry
                    .register_template_string(&format!("{}_html", key), h)
                    .ok();
            }
        }
    }

    /// Register a new template
    pub fn register(
        &mut self,
        key: &str,
        subject_tmpl: &str,
        body_tmpl: &str,
        html_tmpl: Option<&str>,
    ) -> Result<(), ApiError> {
        self.registry
            .register_template_string(&format!("{}_subject", key), subject_tmpl)
            .map_err(|e| ApiError::Internal(format!("Invalid subject template: {}", e)))?;
        self.registry
            .register_template_string(&format!("{}_body", key), body_tmpl)
            .map_err(|e| ApiError::Internal(format!("Invalid body template: {}", e)))?;
        if let Some(html) = html_tmpl {
            self.registry
                .register_template_string(&format!("{}_html", key), html)
                .map_err(|e| ApiError::Internal(format!("Invalid HTML template: {}", e)))?;
        }
        Ok(())
    }

    /// Render a template with variables
    pub fn render(
        &self,
        key: &str,
        vars: &Value,
    ) -> Result<(String, String, Option<String>), ApiError> {
        let subject = self
            .registry
            .render(&format!("{}_subject", key), vars)
            .map_err(|e| {
                ApiError::Internal(format!(
                    "Template render error for subject '{}': {}",
                    key, e
                ))
            })?;
        let body = self
            .registry
            .render(&format!("{}_body", key), vars)
            .map_err(|e| {
                ApiError::Internal(format!("Template render error for body '{}': {}", key, e))
            })?;
        let html = self.registry.render(&format!("{}_html", key), vars).ok();
        Ok((subject, body, html))
    }

    /// Check if a template exists
    pub fn has_template(&self, key: &str) -> bool {
        self.registry.has_template(&format!("{}_subject", key))
    }
}

impl Default for TemplateEngine {
    fn default() -> Self {
        Self::new()
    }
}

/// Built-in email templates (Turkish locale defaults)
fn default_templates() -> Vec<(
    &'static str,
    &'static str,
    &'static str,
    Option<&'static str>,
)> {
    vec![
        (
            "invoice_created",
            "Yeni Fatura: {{invoice_number}}",
            "Sayin {{customer_name}},\n\n{{amount}} {{currency}} tutarinda {{invoice_number}} numarali fatura olusturulmustur.\n\nVade tarihi: {{due_date}}\n\nSaygilarimizla,\nTurerp ERP",
            None,
        ),
        (
            "payment_received",
            "Odeme Alindi: {{payment_id}}",
            "Sayin {{customer_name}},\n\n{{amount}} {{currency}} tutarindaki odemeniz alinmistir.\n\nOdeme tarihi: {{payment_date}}\n\nSaygilarimizla,\nTurerp ERP",
            None,
        ),
        (
            "employee_hired",
            "Yeni Calisan: {{employee_name}}",
            "{{employee_name}} {{department}} departmanina atanmistir.\n\nBaslangic tarihi: {{start_date}}\n\nIK Departmani",
            None,
        ),
        (
            "stock_low",
            "Dusuk Stok Uyarisi: {{product_name}}",
            "{{warehouse_name}} deposunda {{product_name}} urununun stok miktari {{quantity}} seviyesine dusmustur.\n\nMinimum stok seviyesi: {{min_stock}}\n\nStok Yonetimi",
            None,
        ),
        (
            "password_reset",
            "Sifre Sifirlama",
            "Sifrenizi sifirlamak icin asagidaki baglantiyi kullanin:\n\n{{reset_link}}\n\nBu baglanti {{expiry_minutes}} dakika gecerlidir.\n\nTurerp ERP",
            None,
        ),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_template_engine_new() {
        let engine = TemplateEngine::new();
        assert!(engine.has_template("invoice_created"));
    }

    #[test]
    fn test_render_builtin_template() {
        let engine = TemplateEngine::new();
        let vars = serde_json::json!({
            "customer_name": "Acme Corp",
            "invoice_number": "INV-001",
            "amount": "1000.00",
            "currency": "TRY",
            "due_date": "2024-02-01"
        });
        let (subject, body, html) = engine.render("invoice_created", &vars).unwrap();
        assert!(subject.contains("INV-001"));
        assert!(body.contains("Acme Corp"));
        assert!(body.contains("1000.00"));
        assert!(html.is_none());
    }

    #[test]
    fn test_register_and_render_custom_template() {
        let mut engine = TemplateEngine::new();
        engine
            .register(
                "custom",
                "Hello {{name}}",
                "Dear {{name}},\n\n{{message}}",
                Some("<p>Dear {{name}},</p><p>{{message}}</p>"),
            )
            .unwrap();

        let vars = serde_json::json!({
            "name": "Alice",
            "message": "Welcome!"
        });
        let (subject, body, html) = engine.render("custom", &vars).unwrap();
        assert_eq!(subject, "Hello Alice");
        assert!(body.contains("Alice"));
        assert!(html.is_some());
        assert!(html.unwrap().contains("Alice"));
    }

    #[test]
    fn test_missing_template() {
        let engine = TemplateEngine::new();
        let vars = serde_json::json!({"name": "Test"});
        let result = engine.render("nonexistent", &vars);
        assert!(result.is_err());
    }
}
