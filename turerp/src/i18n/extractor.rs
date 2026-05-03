//! Actix-web extractor for the client's preferred locale.
//!
//! Reads the standard `Accept-Language` HTTP header, parses quality values,
//! and returns the best supported locale.

use actix_web::{FromRequest, HttpRequest};

use crate::i18n::{DEFAULT_LOCALE, SUPPORTED_LOCALES};

/// Parsed locale preference from the incoming request.
#[derive(Debug, Clone)]
pub struct Locale {
    inner: String,
}

impl Locale {
    /// Return the locale code, e.g. `"en"` or `"tr"`.
    pub fn as_str(&self) -> &str {
        &self.inner
    }
}

impl TryFrom<&str> for Locale {
    type Error = ();

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        let base = value.split_once('-').map(|(b, _)| b).unwrap_or(value);
        let lowered = base.trim().to_lowercase();
        if SUPPORTED_LOCALES.contains(&lowered.as_str()) {
            Ok(Self { inner: lowered })
        } else {
            Err(())
        }
    }
}

impl FromRequest for Locale {
    type Error = actix_web::Error;
    type Future = std::future::Ready<Result<Self, Self::Error>>;

    fn from_request(req: &HttpRequest, _payload: &mut actix_web::dev::Payload) -> Self::Future {
        let preferred = req
            .headers()
            .get("Accept-Language")
            .and_then(|v| v.to_str().ok())
            .and_then(parse_accept_language)
            .unwrap_or_else(|| DEFAULT_LOCALE.to_string());

        std::future::ready(Ok(Self { inner: preferred }))
    }
}

/// Parse an `Accept-Language` header value such as `"en-US,tr;q=0.9,fr;q=0.5"`
/// and return the best supported locale.
pub fn parse_accept_language(header: &str) -> Option<String> {
    let mut candidates: Vec<(f32, String)> = header
        .split(',')
        .filter_map(|entry| {
            let mut parts = entry.trim().split(';');
            let lang = parts.next()?.trim();
            let base = lang.split_once('-').map(|(b, _)| b).unwrap_or(lang);
            let base_lower = base.trim().to_lowercase();

            // Extract quality value (default 1.0)
            let q = parts
                .next()
                .map(|q_str| {
                    q_str
                        .trim()
                        .strip_prefix("q=")
                        .and_then(|v| v.parse::<f32>().ok())
                        .unwrap_or(1.0)
                })
                .unwrap_or(1.0);

            Some((q, base_lower))
        })
        .collect();

    // Sort by quality descending
    candidates.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));

    // Return first supported locale
    candidates
        .into_iter()
        .find(|(_, l)| SUPPORTED_LOCALES.contains(&l.as_str()))
        .map(|(_, l)| l)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_accept_language_simple() {
        assert_eq!(parse_accept_language("en"), Some("en".to_string()));
    }

    #[test]
    fn test_parse_accept_language_with_quality() {
        assert_eq!(
            parse_accept_language("en-US,tr;q=0.9"),
            Some("en".to_string())
        );
    }

    #[test]
    fn test_parse_accept_language_turkish_first() {
        assert_eq!(
            parse_accept_language("tr-TR,en;q=0.8"),
            Some("tr".to_string())
        );
    }

    #[test]
    fn test_parse_accept_language_unsupported_fallback() {
        // German is unsupported, should fall through to English
        assert_eq!(parse_accept_language("de,en;q=0.5"), Some("en".to_string()));
    }

    #[test]
    fn test_parse_accept_language_tr_over_en() {
        // Turkish has higher quality
        assert_eq!(
            parse_accept_language("tr;q=0.9,en;q=0.8"),
            Some("tr".to_string())
        );
    }

    #[test]
    fn test_parse_accept_language_empty_returns_none() {
        assert_eq!(parse_accept_language(""), None);
        assert_eq!(parse_accept_language("de,fr"), None); // no supported locales
    }

    #[test]
    fn test_locale_try_from() {
        let locale: Locale = "en".try_into().unwrap();
        assert_eq!(locale.as_str(), "en");

        let locale: Locale = "tr-TR".try_into().unwrap();
        assert_eq!(locale.as_str(), "tr");

        assert!(TryInto::<Locale>::try_into("xx").is_err());
    }
}
