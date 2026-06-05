use worker::Url;

pub fn query_param(url: &Url, key: &str) -> Option<String> {
    url.query_pairs()
        .find(|(k, _)| k == key)
        .map(|(_, v)| v.into_owned())
}

pub fn parse_f64_param(url: &Url, key: &str) -> Option<f64> {
    query_param(url, key).and_then(|v| v.parse().ok())
}

pub fn parse_usize_param(url: &Url, key: &str, default: usize) -> usize {
    query_param(url, key)
        .and_then(|v| v.parse().ok())
        .unwrap_or(default)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_query_param_parsing() {
        let url = Url::parse("https://example.com/api?lat=-6.175&lon=106.825&limit=10").unwrap();

        assert_eq!(query_param(&url, "lat"), Some("-6.175".to_string()));
        assert_eq!(query_param(&url, "lon"), Some("106.825".to_string()));
        assert_eq!(query_param(&url, "limit"), Some("10".to_string()));
        assert_eq!(query_param(&url, "missing"), None);
    }

    #[test]
    fn test_parse_f64_param() {
        let url = Url::parse("https://example.com/api?lat=-6.175&lon=106.825").unwrap();

        assert!((parse_f64_param(&url, "lat").unwrap() - (-6.175)).abs() < 0.001);
        assert!((parse_f64_param(&url, "lon").unwrap() - 106.825).abs() < 0.001);
        assert_eq!(parse_f64_param(&url, "missing"), None);
    }

    #[test]
    fn test_parse_f64_param_invalid() {
        let url = Url::parse("https://example.com/api?lat=not_a_number").unwrap();
        assert_eq!(parse_f64_param(&url, "lat"), None);
    }

    #[test]
    fn test_parse_usize_param() {
        let url = Url::parse("https://example.com/api?limit=25").unwrap();

        assert_eq!(parse_usize_param(&url, "limit", 10), 25);
        assert_eq!(parse_usize_param(&url, "missing", 5), 5);
    }

    #[test]
    fn test_parse_usize_param_invalid_uses_default() {
        let url = Url::parse("https://example.com/api?limit=not_a_number").unwrap();
        assert_eq!(parse_usize_param(&url, "limit", 10), 10);
    }
}
