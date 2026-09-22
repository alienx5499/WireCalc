use super::headers::HttpHeaders;
use super::method::HttpMethod;
use super::request::HttpRequest;
use super::version::HttpVersion;
use std::str;

#[derive(Debug, PartialEq, Eq)]
pub enum ParseStatus {
    Complete(HttpRequest, usize),
    NeedMoreData,
    RecoverableError {
        error: String,
        consumed_bytes: usize,
        close_connection: bool,
    },
    FatalError(String),
}

pub struct HttpParser;

impl HttpParser {
    #[inline]
    fn find_header_delimiter(buf: &[u8]) -> Option<(usize, usize)> {
        let len = buf.len();
        if len < 2 {
            return None;
        }
        for i in 0..len - 1 {
            if buf[i] == b'\r' && i + 3 < len && &buf[i..i + 4] == b"\r\n\r\n" {
                return Some((i, 4));
            }
            if buf[i] == b'\n' && buf[i + 1] == b'\n' {
                return Some((i, 2));
            }
        }
        None
    }

    /// Attempts to parse an HTTP request from the buffer.
    /// Handles framing, Content-Length boundaries, and RFC 7230 Host verification.
    pub fn parse(buf: &[u8]) -> ParseStatus {
        let (header_end, delim_len) = match Self::find_header_delimiter(buf) {
            Some(res) => res,
            None => return ParseStatus::NeedMoreData,
        };

        let Ok(header_str) = str::from_utf8(&buf[..header_end]) else {
            return ParseStatus::FatalError("Invalid UTF-8 encoding in headers".into());
        };

        let mut lines = header_str.lines();
        let Some(request_line) = lines.next().map(str::trim).filter(|l| !l.is_empty()) else {
            return ParseStatus::FatalError("Empty request line".into());
        };

        let mut parts = request_line.split_whitespace();
        let (Some(method_str), Some(uri)) = (parts.next(), parts.next()) else {
            return ParseStatus::FatalError("Malformed request line".into());
        };
        let version = HttpVersion::from_str_bytes(parts.next().unwrap_or("HTTP/1.1"));
        let method = HttpMethod::from_str_bytes(method_str);

        let mut headers = HttpHeaders::new();
        for line in lines {
            let line = line.trim();
            if !line.is_empty()
                && let Some(colon_idx) = line.find(':')
            {
                headers.insert(&line[..colon_idx], &line[colon_idx + 1..]);
            }
        }

        let body_start = header_end + delim_len;
        let content_length = headers.content_length().unwrap_or(0);
        let total_consumed = body_start + content_length;

        if buf.len() < total_consumed {
            return ParseStatus::NeedMoreData;
        }

        let close_connection = headers.is_connection_close();
        if version == HttpVersion::Http11 && !headers.has_valid_host() {
            return ParseStatus::RecoverableError {
                error: "HTTP/1.1 request missing Host header".into(),
                consumed_bytes: total_consumed,
                close_connection,
            };
        }

        let body = if content_length > 0 {
            buf[body_start..total_consumed].to_vec()
        } else {
            Vec::new()
        };

        let request = HttpRequest::new(method, uri.to_string(), version, headers, body);
        ParseStatus::Complete(request, total_consumed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_get() {
        let wire = b"GET /add?a=2&b=3 HTTP/1.1\r\nHost: localhost:8080\r\n\r\n";
        let (req, consumed) = match HttpParser::parse(wire) {
            ParseStatus::Complete(r, c) => (r, c),
            other => panic!("Expected Complete, got {other:?}"),
        };
        assert_eq!(consumed, wire.len());
        assert_eq!(req.method, HttpMethod::Get);
        assert_eq!(req.path(), "/add");
        assert_eq!(req.get_query("a"), Some("2"));
        assert_eq!(req.get_query("b"), Some("3"));
    }

    #[test]
    fn test_parse_missing_host() {
        let wire = b"GET /add?a=2&b=3 HTTP/1.1\r\nUser-Agent: test\r\n\r\n";
        match HttpParser::parse(wire) {
            ParseStatus::RecoverableError {
                consumed_bytes,
                error,
                ..
            } => {
                assert_eq!(consumed_bytes, wire.len());
                assert!(error.contains("missing Host"));
            }
            other => panic!("Expected RecoverableError, got {other:?}"),
        }
    }

    #[test]
    fn test_pipelining_buffer_slicing() {
        let req1 = b"GET /add?a=2&b=3 HTTP/1.1\r\nHost: localhost:8080\r\n\r\n";
        let req2 = b"GET /sub?a=10&b=4 HTTP/1.1\r\nHost: localhost:8080\r\n\r\n";
        let mut combined = [req1.as_slice(), req2.as_slice()].concat();

        let (req1_res, c1) = match HttpParser::parse(&combined) {
            ParseStatus::Complete(r, c) => (r, c),
            other => panic!("Expected Complete for req1, got {other:?}"),
        };
        assert_eq!(req1_res.path(), "/add");
        assert_eq!(c1, req1.len());
        combined.drain(..c1);

        let (req2_res, c2) = match HttpParser::parse(&combined) {
            ParseStatus::Complete(r, c) => (r, c),
            other => panic!("Expected Complete for req2, got {other:?}"),
        };
        assert_eq!(req2_res.path(), "/sub");
        assert_eq!(c2, req2.len());
    }

    #[test]
    fn test_content_length_exact_boundary() {
        let body = b"{\"hello\":\"world\"}";
        let raw = format!(
            "POST /add HTTP/1.1\r\nHost: localhost\r\nContent-Length: {}\r\n\r\n{}",
            body.len(),
            std::str::from_utf8(body).unwrap()
        );
        let mut wire = raw.into_bytes();
        wire.extend_from_slice(b"G");

        let (req, consumed) = match HttpParser::parse(&wire) {
            ParseStatus::Complete(r, c) => (r, c),
            other => panic!("Expected Complete, got {other:?}"),
        };
        assert_eq!(req.body, body);
        assert_eq!(consumed, wire.len() - 1);
    }

    #[test]
    fn test_post_body_followed_by_get() {
        let post_req = b"POST /add HTTP/1.1\r\nHost: localhost\r\nContent-Length: 5\r\n\r\nHELLO";
        let get_req = b"GET /add?a=2&b=3 HTTP/1.1\r\nHost: localhost\r\n\r\n";
        let mut combined = [post_req.as_slice(), get_req.as_slice()].concat();

        let (req1, c1) = match HttpParser::parse(&combined) {
            ParseStatus::Complete(r, c) => (r, c),
            other => panic!("Expected Complete for POST, got {other:?}"),
        };
        assert_eq!(req1.method, HttpMethod::Post);
        assert_eq!(req1.body, b"HELLO");
        assert_eq!(c1, post_req.len());
        combined.drain(..c1);

        let (req2, c2) = match HttpParser::parse(&combined) {
            ParseStatus::Complete(r, c) => (r, c),
            other => panic!("Expected Complete for GET, got {other:?}"),
        };
        assert_eq!(req2.method, HttpMethod::Get);
        assert_eq!(req2.path(), "/add");
        assert_eq!(c2, get_req.len());
    }
}
