//! Client IP extraction utilities for middleware
//!
//! Provides shared helper functions for extracting client IP addresses
//! from HTTP requests, with support for trusted proxies and
//! `X-Forwarded-For` / `X-Real-IP` headers.

use actix_web::dev::ServiceRequest;
use std::net::IpAddr;

/// Check if a peer IP is a loopback address.
///
/// Loopback addresses are always trusted (useful for local development).
pub fn is_loopback(peer_ip: &str) -> bool {
    let Ok(parsed) = peer_ip.parse::<IpAddr>() else {
        return false;
    };
    parsed.is_loopback()
}

/// Check if a peer IP is in the trusted proxies list.
pub fn is_in_trusted_proxies(peer_ip: &str, trusted_proxies: &[IpAddr]) -> bool {
    let Ok(parsed) = peer_ip.parse::<IpAddr>() else {
        return false;
    };

    trusted_proxies.contains(&parsed)
}

/// Extract the real client IP from request headers, considering trusted proxies.
///
/// Logic:
/// 1. Get `peer_ip` from `req.connection_info().peer_addr()`.
/// 2. If `trusted_proxies` is non-empty, trust headers when peer is in the list;
///    otherwise trust headers when peer is loopback.
/// 3. If trusted, read `X-Forwarded-For` (left-most, comma-split) then `X-Real-IP`.
///    Each extracted IP is validated with `parse::<IpAddr>()`; invalid values log a
///    warning and fall back to the next source.
/// 4. If not trusted or no forwarding headers, fall back to `peer_ip`.
pub fn extract_client_ip(req: &ServiceRequest, trusted_proxies: &[IpAddr]) -> Option<String> {
    let peer_ip = req
        .connection_info()
        .peer_addr()
        .unwrap_or("unknown")
        .to_string();

    let may_trust_headers = if !trusted_proxies.is_empty() {
        is_in_trusted_proxies(&peer_ip, trusted_proxies)
    } else {
        is_loopback(&peer_ip)
    };

    if may_trust_headers {
        // Check X-Forwarded-For first
        if let Some(forwarded) = req.headers().get("X-Forwarded-For") {
            if let Ok(forwarded_str) = forwarded.to_str() {
                if let Some(client_ip) = forwarded_str.split(',').next() {
                    let trimmed = client_ip.trim().to_string();
                    if !trimmed.is_empty() {
                        // Validate extracted IP format to prevent injection
                        if trimmed.parse::<IpAddr>().is_ok() {
                            return Some(trimmed);
                        }
                        tracing::warn!(
                            peer_ip = %peer_ip,
                            forwarded = %trimmed,
                            "Invalid IP format in X-Forwarded-For, falling back to peer IP"
                        );
                    }
                }
            }
        }

        // Fall back to X-Real-IP
        if let Some(real_ip) = req.headers().get("X-Real-IP") {
            if let Ok(ip) = real_ip.to_str() {
                let trimmed = ip.trim().to_string();
                if !trimmed.is_empty() {
                    // Validate extracted IP format to prevent injection
                    if trimmed.parse::<IpAddr>().is_ok() {
                        return Some(trimmed);
                    }
                    tracing::warn!(
                        peer_ip = %peer_ip,
                        real_ip = %trimmed,
                        "Invalid IP format in X-Real-IP, falling back to peer IP"
                    );
                }
            }
        }
    }

    // Direct peer IP (not behind a trusted proxy or no forwarding headers)
    req.connection_info().peer_addr().map(|s| s.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_loopback() {
        assert!(is_loopback("127.0.0.1"));
        assert!(is_loopback("::1"));
    }

    #[test]
    fn test_is_not_loopback() {
        assert!(!is_loopback("192.168.1.1"));
        assert!(!is_loopback("10.0.0.1"));
        assert!(!is_loopback("unknown"));
    }

    #[test]
    fn test_is_in_trusted_proxies() {
        let proxies: Vec<IpAddr> = vec!["10.0.0.1".parse().unwrap(), "10.0.0.2".parse().unwrap()];
        assert!(is_in_trusted_proxies("10.0.0.1", &proxies));
        assert!(is_in_trusted_proxies("10.0.0.2", &proxies));
        assert!(!is_in_trusted_proxies("10.0.0.3", &proxies));
        assert!(!is_in_trusted_proxies("invalid", &proxies));
    }
}
