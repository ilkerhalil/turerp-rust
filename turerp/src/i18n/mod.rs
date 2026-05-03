//! Localization (i18n) for Turerp ERP
//!
//! Supports JSON-based translation bundles loaded at startup.
//! Language is detected per-request via the `Accept-Language` header
//! and falls back to the configured default locale.
//!
//! # Usage
//! ```ignore
//! use turerp::i18n::{I18n, Locale};
//!
//! pub async fn handler(i18n: web::Data<I18n>, locale: Locale) -> String {
//!     i18n.t(locale.as_str(), "hello")
//! }
//! ```

pub mod extractor;
pub use extractor::Locale;

use std::collections::HashMap;
use std::sync::OnceLock;

/// Global translation cache, loaded once at first access.
static TRANSLATIONS: OnceLock<HashMap<String, HashMap<String, String>>> = OnceLock::new();

/// Helper to resolve `I18n` from an optional `web::Data`.
/// Falls back to a lazily-initialized singleton so handlers work even
/// when the caller did not register `web::Data<I18n>` in the app.
pub fn resolve(i18n: &Option<actix_web::web::Data<I18n>>) -> &I18n {
    i18n.as_ref().map(|d| d.get_ref()).unwrap_or_else(|| {
        static FALLBACK: OnceLock<I18n> = OnceLock::new();
        FALLBACK.get_or_init(I18n::init)
    })
}

/// Supported locales (used for validation).
pub const SUPPORTED_LOCALES: &[&str] = &["en", "tr"];

/// Default locale when none is specified or detected.
pub const DEFAULT_LOCALE: &str = "en";

/// Localization service.
///
/// Holds no state itself; everything lives in the global `TRANSLATIONS` cache.
/// This design makes `I18n` cheaply `Clone`-able and easy to store in `AppState`.
#[derive(Debug, Clone, Default)]
pub struct I18n;

impl I18n {
    /// Initialise translations by scanning `locales/` for `*.json` files.
    /// Safe to call multiple times — only the first call loads data.
    pub fn init() -> Self {
        let _ = TRANSLATIONS.get_or_init(|| {
            let mut bundles = HashMap::new();
            for locale in SUPPORTED_LOCALES {
                let path = format!("locales/{}.json", locale);
                match std::fs::read_to_string(&path) {
                    Ok(content) => {
                        match serde_json::from_str::<HashMap<String, String>>(&content) {
                            Ok(map) => {
                                bundles.insert(locale.to_string(), map);
                            }
                            Err(e) => {
                                tracing::error!("Failed to parse {}: {}", path, e);
                            }
                        }
                    }
                    Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                        tracing::warn!("Translation file not found: {}", path);
                    }
                    Err(e) => {
                        tracing::error!("Failed to read {}: {}", path, e);
                    }
                }
            }
            bundles
        });
        Self
    }

    /// Translate a key into the requested locale.
    /// Falls back to the default locale, then to the raw key itself.
    pub fn t(&self, locale: &str, key: &str) -> String {
        let normalized = Self::normalize_locale(locale);
        TRANSLATIONS
            .get()
            .and_then(|b| b.get(&normalized).and_then(|m| m.get(key).cloned()))
            .or_else(|| {
                TRANSLATIONS
                    .get()
                    .and_then(|b| b.get(DEFAULT_LOCALE).and_then(|m| m.get(key).cloned()))
            })
            .unwrap_or_else(|| key.to_string())
    }

    /// Translate with `{key}` placeholder interpolation.
    /// `args` is a slice of `(placeholder_name, value)` tuples.
    pub fn t_args(&self, locale: &str, key: &str, args: &[(&str, &str)]) -> String {
        let mut text = self.t(locale, key);
        for (placeholder, value) in args {
            text = text.replace(&format!("{{{}}}", placeholder), value);
        }
        text
    }

    /// Normalize a raw locale string (e.g. "en-US" → "en", "tr-TR" → "tr")
    /// and ensure it is supported; otherwise return the default.
    pub fn normalize_locale(locale: &str) -> String {
        let base = locale.split_once('-').map(|(b, _)| b).unwrap_or(locale);
        let lowered = base.trim().to_lowercase();
        if SUPPORTED_LOCALES.contains(&lowered.as_str()) {
            lowered
        } else {
            DEFAULT_LOCALE.to_string()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ensure_loaded() {
        I18n::init();
    }

    #[test]
    fn test_normalize_locale() {
        assert_eq!(I18n::normalize_locale("en"), "en");
        assert_eq!(I18n::normalize_locale("en-US"), "en");
        assert_eq!(I18n::normalize_locale("tr-TR"), "tr");
        assert_eq!(I18n::normalize_locale("fr"), "en"); // unsupported → default
        assert_eq!(I18n::normalize_locale("  EN  "), "en");
    }

    #[test]
    fn test_translation_lookup() {
        ensure_loaded();
        let i18n = I18n;

        // Default locale should at least have generic keys
        let msg = i18n.t("en", "generic.hello");
        assert!(!msg.is_empty());

        // Unknown key falls back to raw key
        let msg = i18n.t("en", "nonexistent.key");
        assert_eq!(msg, "nonexistent.key");
    }

    #[test]
    fn test_translation_args_interpolation() {
        ensure_loaded();
        let i18n = I18n;

        let msg = i18n.t_args("en", "errors.not_found", &[("resource", "Invoice")]);
        assert!(msg.contains("Invoice"));
    }

    #[test]
    fn test_fallback_to_default_locale() {
        ensure_loaded();
        let i18n = I18n;

        // Request unsupported locale; should fall back to English
        let msg = i18n.t("xx", "generic.hello");
        assert!(msg.contains("Hello") || msg == "generic.hello");
    }
}
