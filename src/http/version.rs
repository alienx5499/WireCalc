use std::fmt;

/// Supported HTTP protocol versions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum HttpVersion {
    #[default]
    Http11,
    Http10,
    Other,
}

impl HttpVersion {
    /// Fast parse from string slice without heap allocations.
    pub fn from_str_bytes(s: &str) -> Self {
        match s {
            "HTTP/1.1" => HttpVersion::Http11,
            "HTTP/1.0" => HttpVersion::Http10,
            _ => HttpVersion::Other,
        }
    }

    #[inline]
    pub fn as_str(&self) -> &'static str {
        match self {
            HttpVersion::Http11 => "HTTP/1.1",
            HttpVersion::Http10 => "HTTP/1.0",
            HttpVersion::Other => "HTTP/Other",
        }
    }
}

impl fmt::Display for HttpVersion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}
