/// HTTP headers collection with fast-path indexing for critical HTTP/1.1 headers.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct HttpHeaders {
    host: Option<String>,
    content_length: Option<usize>,
    connection_close: bool,
    other: Vec<(String, String)>,
}

impl HttpHeaders {
    pub fn new() -> Self {
        Self {
            host: None,
            content_length: None,
            connection_close: false,
            other: Vec::with_capacity(4),
        }
    }

    /// Insert or append a header name/value pair with fast-path categorization.
    pub fn insert(&mut self, name: &str, value: &str) {
        let name_trimmed = name.trim();
        let val_trimmed = value.trim();

        if name_trimmed.eq_ignore_ascii_case("host") {
            self.host = Some(val_trimmed.to_string());
        } else if name_trimmed.eq_ignore_ascii_case("content-length") {
            self.content_length = val_trimmed.parse::<usize>().ok();
        } else if name_trimmed.eq_ignore_ascii_case("connection") {
            self.connection_close = val_trimmed.eq_ignore_ascii_case("close");
        } else {
            self.other
                .push((name_trimmed.to_ascii_lowercase(), val_trimmed.to_string()));
        }
    }

    /// Look up an arbitrary header by case-insensitive name.
    #[allow(dead_code)]
    pub fn get(&self, name: &str) -> Option<&str> {
        if name.eq_ignore_ascii_case("host") {
            return self.host.as_deref();
        }
        if name.eq_ignore_ascii_case("connection") {
            return if self.connection_close {
                Some("close")
            } else {
                Some("keep-alive")
            };
        }
        let lower = name.to_ascii_lowercase();
        for (k, v) in &self.other {
            if k == &lower {
                return Some(v.as_str());
            }
        }
        None
    }

    /// Check if Host header exists and is non-empty (RFC 7230 Section 5.4 requirement).
    #[inline]
    pub fn has_valid_host(&self) -> bool {
        match &self.host {
            Some(val) => !val.is_empty(),
            None => false,
        }
    }

    /// Fast pre-parsed Content-Length lookup.
    #[inline]
    pub fn content_length(&self) -> Option<usize> {
        self.content_length
    }

    /// Fast O(1) check if the client requested connection closure via 'Connection: close'.
    #[inline]
    pub fn is_connection_close(&self) -> bool {
        self.connection_close
    }
}
