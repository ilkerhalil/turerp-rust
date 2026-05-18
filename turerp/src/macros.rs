//! Shared macros to reduce handler boilerplate

/// Wrap a service call into a JSON HTTP response.
///
/// Reduces the repetitive `match service.call().await { Ok => json, Err => error }`
/// pattern seen in almost every handler.
///
/// # Usage
/// ```ignore
/// json_resp!(invoice_service.create_invoice(create), HttpResponse::Created, i18n, locale.as_str())
/// ```
#[macro_export]
macro_rules! json_resp {
    ($expr:expr, $status:path, $i18n:expr, $locale:expr) => {
        match $expr.await {
            Ok(response) => Ok($status().json(response)),
            Err(e) => Ok(e.to_http_response($i18n, $locale)),
        }
    };
}
