use super::headers::HttpHeaders;
use super::method::HttpMethod;
use super::version::HttpVersion;

/// Represents a parsed HTTP request with zero-allocation URI views.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HttpRequest {
    pub method: HttpMethod,
    pub raw_uri: String,
    query_pos: Option<usize>,
    pub version: HttpVersion,
    pub headers: HttpHeaders,
    pub body: Vec<u8>,
}

impl HttpRequest {
    pub fn new(
        method: HttpMethod,
        raw_uri: String,
        version: HttpVersion,
        headers: HttpHeaders,
        body: Vec<u8>,
    ) -> Self {
        let query_pos = raw_uri.find('?');
        Self {
            method,
            raw_uri,
            query_pos,
            version,
            headers,
            body,
        }
    }

    /// Zero-copy path slice without heap allocation.
    #[inline]
    pub fn path(&self) -> &str {
        match self.query_pos {
            Some(pos) => &self.raw_uri[..pos],
            None => &self.raw_uri,
        }
    }

    /// Zero-copy query string slice without heap allocation.
    #[inline]
    pub fn query_str(&self) -> Option<&str> {
        self.query_pos.map(|pos| &self.raw_uri[pos + 1..])
    }

    /// Zero-allocation query parameter lookup directly on the query string slice.
    pub fn get_query(&self, target_key: &str) -> Option<&str> {
        let q = self.query_str()?;
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
        self.headers.is_connection_close() || self.version == HttpVersion::Http10
    }
}
