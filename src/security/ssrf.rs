use reqwest::Url;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, ToSocketAddrs};

/// Returns true if the provided IPv4 address is in a private, loopback, link-local,
/// cloud metadata, multicast, or otherwise reserved/restricted range.
pub fn is_restricted_ipv4(ip: Ipv4Addr) -> bool {
    let octets = ip.octets();

    // 0.0.0.0/8: "This network" / Unspecified
    if octets[0] == 0 {
        return true;
    }

    // 10.0.0.0/8: Private network (RFC 1918)
    if octets[0] == 10 {
        return true;
    }

    // 100.64.0.0/10: Shared address space / CGNAT (RFC 6598)
    if octets[0] == 100 && (64..=127).contains(&octets[1]) {
        return true;
    }

    // 127.0.0.0/8: Loopback addresses (RFC 1122)
    if octets[0] == 127 {
        return true;
    }

    // 169.254.0.0/16: Link-local / Cloud metadata (RFC 3927, e.g. 169.254.169.254)
    if octets[0] == 169 && octets[1] == 254 {
        return true;
    }

    // 172.16.0.0/12: Private network (RFC 1918: 172.16.0.0 - 172.31.255.255)
    if octets[0] == 172 && (16..=31).contains(&octets[1]) {
        return true;
    }

    // 192.0.0.0/24: IETF Protocol Assignments (RFC 6890)
    if octets[0] == 192 && octets[1] == 0 && octets[2] == 0 {
        return true;
    }

    // 192.0.2.0/24: Documentation TEST-NET-1 (RFC 5737)
    if octets[0] == 192 && octets[1] == 0 && octets[2] == 2 {
        return true;
    }

    // 192.88.99.0/24: 6to4 Relay Anycast (RFC 7526)
    if octets[0] == 192 && octets[1] == 88 && octets[2] == 99 {
        return true;
    }

    // 192.168.0.0/16: Private network (RFC 1918)
    if octets[0] == 192 && octets[1] == 168 {
        return true;
    }

    // 198.18.0.0/15: Network benchmark tests (RFC 2544)
    if octets[0] == 198 && (18..=19).contains(&octets[1]) {
        return true;
    }

    // 198.51.100.0/24: Documentation TEST-NET-2 (RFC 5737)
    if octets[0] == 198 && octets[1] == 51 && octets[2] == 100 {
        return true;
    }

    // 203.0.113.0/24: Documentation TEST-NET-3 (RFC 5737)
    if octets[0] == 203 && octets[1] == 0 && octets[2] == 113 {
        return true;
    }

    // 224.0.0.0/4: Multicast (RFC 5771)
    if (224..=239).contains(&octets[0]) {
        return true;
    }

    // 240.0.0.0/4: Reserved for future use & Broadcast 255.255.255.255 (RFC 1112)
    if octets[0] >= 240 {
        return true;
    }

    false
}

/// Returns true if the provided IPv6 address is in a private, loopback, link-local,
/// multicast, documentation, or IPv4-mapped/compatible restricted range.
pub fn is_restricted_ipv6(ip: Ipv6Addr) -> bool {
    let segments = ip.segments();

    // ::1: Loopback (RFC 4291)
    if ip.is_loopback() {
        return true;
    }

    // ::: Unspecified (RFC 4291)
    if ip.is_unspecified() {
        return true;
    }

    // fe80::/10: Link-local unicast (RFC 4291)
    if (segments[0] & 0xffc0) == 0xfe80 {
        return true;
    }

    // fc00::/7: Unique local address (ULA / Private, RFC 4193)
    if (segments[0] & 0xfe00) == 0xfc00 {
        return true;
    }

    // ff00::/8: Multicast (RFC 4291)
    if (segments[0] & 0xff00) == 0xff00 {
        return true;
    }

    // 2001:db8::/32: Documentation (RFC 3849)
    if segments[0] == 0x2001 && segments[1] == 0x0db8 {
        return true;
    }

    // 100::/64: Discard-only prefix (RFC 6666)
    if segments[0] == 0x0100 && segments[1] == 0 && segments[2] == 0 && segments[3] == 0 {
        return true;
    }

    // 2002::/16: 6to4 encapsulation (RFC 3056) -> check embedded IPv4
    if segments[0] == 0x2002 {
        let embedded_v4 = Ipv4Addr::new(
            (segments[1] >> 8) as u8,
            (segments[1] & 0xff) as u8,
            (segments[2] >> 8) as u8,
            (segments[2] & 0xff) as u8,
        );
        if is_restricted_ipv4(embedded_v4) {
            return true;
        }
    }

    // IPv4-mapped IPv6 address: ::ffff:a.b.c.d
    if segments[0] == 0
        && segments[1] == 0
        && segments[2] == 0
        && segments[3] == 0
        && segments[4] == 0
        && segments[5] == 0xffff
    {
        let embedded_v4 = Ipv4Addr::new(
            (segments[6] >> 8) as u8,
            (segments[6] & 0xff) as u8,
            (segments[7] >> 8) as u8,
            (segments[7] & 0xff) as u8,
        );
        return is_restricted_ipv4(embedded_v4);
    }

    // IPv4-compatible IPv6 address: ::a.b.c.d
    if segments[0] == 0
        && segments[1] == 0
        && segments[2] == 0
        && segments[3] == 0
        && segments[4] == 0
        && segments[5] == 0
        && (segments[6] != 0 || segments[7] > 1)
    {
        let embedded_v4 = Ipv4Addr::new(
            (segments[6] >> 8) as u8,
            (segments[6] & 0xff) as u8,
            (segments[7] >> 8) as u8,
            (segments[7] & 0xff) as u8,
        );
        return is_restricted_ipv4(embedded_v4);
    }

    false
}

/// Checks if an IP address (v4 or v6) is private or restricted.
pub fn is_private_or_restricted_ip(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(v4) => is_restricted_ipv4(v4),
        IpAddr::V6(v6) => is_restricted_ipv6(v6),
    }
}

/// Checks if a hostname or domain name is restricted (e.g. localhost, cloud metadata, local/internal domains).
pub fn is_restricted_hostname(host: &str) -> bool {
    let lower = host.trim().to_ascii_lowercase();

    // Exact matches
    if lower == "localhost"
        || lower == "localhost.localdomain"
        || lower == "ip6-localhost"
        || lower == "ip6-loopback"
        || lower == "metadata.google.internal"
        || lower == "metadata"
        || lower == "instance-data"
    {
        return true;
    }

    // Suffix matches for internal / private TLDs
    if lower.ends_with(".localhost")
        || lower.ends_with(".local")
        || lower.ends_with(".internal")
        || lower.ends_with(".lan")
        || lower.ends_with(".home")
        || lower.ends_with(".corp")
        || lower.ends_with(".invalid")
        || lower.ends_with(".onion")
    {
        return true;
    }

    false
}

/// Validates a parsed `reqwest::Url` against SSRF risks.
pub fn validate_parsed_url(url: &Url) -> Result<(), String> {
    // 1. Scheme check
    let scheme = url.scheme().to_ascii_lowercase();
    if scheme != "http" && scheme != "https" {
        return Err(format!(
            "Invalid URL scheme '{}'. Only 'http' and 'https' are permitted.",
            scheme
        ));
    }

    // 2. Host presence check
    let host_str = match url.host_str() {
        Some(h) if !h.trim().is_empty() => h.trim(),
        _ => return Err("URL must have a valid host".to_string()),
    };

    // 3. Hostname blacklist check
    if is_restricted_hostname(host_str) {
        return Err(format!(
            "Access to internal or reserved hostname '{}' is blocked.",
            host_str
        ));
    }

    // 4. IP literal check
    if let Ok(ip) = host_str.parse::<IpAddr>() {
        if is_private_or_restricted_ip(ip) {
            return Err(format!(
                "Access to private or restricted IP address '{}' is blocked.",
                ip
            ));
        }
        return Ok(());
    }

    // Strip bracket notation from IPv6 if present
    let clean_host = host_str.trim_start_matches('[').trim_end_matches(']');
    if let Ok(ip) = clean_host.parse::<IpAddr>() {
        if is_private_or_restricted_ip(ip) {
            return Err(format!(
                "Access to private or restricted IP address '{}' is blocked.",
                ip
            ));
        }
        return Ok(());
    }

    // 5. DNS Resolution check
    let port = url.port_or_known_default().unwrap_or(80);
    let socket_addr_str = format!("{}:{}", host_str, port);

    match socket_addr_str.to_socket_addrs() {
        Ok(addrs) => {
            let mut resolved_any = false;
            for addr in addrs {
                resolved_any = true;
                if is_private_or_restricted_ip(addr.ip()) {
                    return Err(format!(
                        "Host '{}' resolved to private or restricted IP address '{}'. Request blocked.",
                        host_str,
                        addr.ip()
                    ));
                }
            }
            if !resolved_any {
                return Err(format!("Could not resolve hostname '{}'.", host_str));
            }
        }
        Err(e) => {
            return Err(format!("DNS resolution failed for '{}': {}", host_str, e));
        }
    }

    Ok(())
}

/// Normalizes and validates a raw URL string for SSRF prevention.
pub fn validate_url_for_ssrf(raw_url: &str) -> Result<Url, String> {
    let clean_url = raw_url.trim();
    if clean_url.is_empty() {
        return Err("URL cannot be empty".to_string());
    }

    let full_url = if !clean_url.starts_with("http://") && !clean_url.starts_with("https://") {
        if clean_url.contains("://") {
            return Err(
                "Disallowed URL scheme. Only 'http' and 'https' are permitted.".to_string(),
            );
        }
        format!("https://{}", clean_url)
    } else {
        clean_url.to_string()
    };

    let parsed_url = Url::parse(&full_url).map_err(|e| format!("Invalid URL format: {}", e))?;

    validate_parsed_url(&parsed_url)?;

    Ok(parsed_url)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_restricted_ipv4_addresses() {
        assert!(is_restricted_ipv4(Ipv4Addr::new(127, 0, 0, 1)));
        assert!(is_restricted_ipv4(Ipv4Addr::new(10, 0, 0, 1)));
        assert!(is_restricted_ipv4(Ipv4Addr::new(172, 16, 0, 1)));
        assert!(is_restricted_ipv4(Ipv4Addr::new(192, 168, 1, 1)));
        assert!(is_restricted_ipv4(Ipv4Addr::new(169, 254, 169, 254)));
        assert!(!is_restricted_ipv4(Ipv4Addr::new(8, 8, 8, 8)));
    }

    #[test]
    fn test_restricted_ipv6_addresses() {
        assert!(is_restricted_ipv6(Ipv6Addr::LOCALHOST));
        assert!(is_restricted_ipv6(Ipv6Addr::UNSPECIFIED));
    }

    #[test]
    fn test_restricted_hostnames() {
        assert!(is_restricted_hostname("localhost"));
        assert!(is_restricted_hostname("metadata.google.internal"));
        assert!(!is_restricted_hostname("google.com"));
    }
}
