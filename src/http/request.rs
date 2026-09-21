use super::headers::HttpHeaders;
use super::method::HttpMethod;

/// Represents a parsed HTTP/1.1 request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HttpRequest {
    pub method: HttpMethod,
    pub raw_uri: String,
    pub path: String,
    pub query_str: Option<String>,
    pub version: String,
    pub headers: HttpHeaders,
    pub body: Vec<u8>,
}

impl HttpRequest {
    pub fn new(
        method: HttpMethod,
        raw_uri: String,
        version: String,
        headers: HttpHeaders,
        body: Vec<u8>,
    ) -> Self {
        let (path, query_str) = match raw_uri.find('?') {
            Some(pos) => (raw_uri[..pos].to_string(), Some(raw_uri[pos + 1..].to_string())),
            None => (raw_uri.clone(), None),
        };

        Self {
            method,
            raw_uri,
            path,
            query_str,
            version,
            headers,
            body,
        }
    }

    /// Zero-allocation query parameter lookup directly on the query string slice.
    pub fn get_query(&self, target_key: &str) -> Option<&str> {
        let q = self.query_str.as_deref()?;
        for pair in q.split('&') {
            if let Some((k, v)) = pair.split_once('=')
                && k == target_key
            {
                return Some(v);
            }
        }
        None
    }

    /// Determine if connection should close based on headers or version.
    #[inline]
    pub fn should_close(&self) -> bool {
        self.headers.is_connection_close() || self.version == "HTTP/1.0"
    }
}
