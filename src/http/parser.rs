use super::headers::HttpHeaders;
use super::method::HttpMethod;
use super::request::HttpRequest;
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
        let mut i = 0;
        while i < len - 1 {
            if buf[i] == b'\r' && i + 3 < len && &buf[i..i + 4] == b"\r\n\r\n" {
                return Some((i, 4));
            }
            if buf[i] == b'\n' && buf[i + 1] == b'\n' {
                return Some((i, 2));
            }
            i += 1;
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

        let header_str = match str::from_utf8(&buf[..header_end]) {
            Ok(s) => s,
            Err(_) => return ParseStatus::FatalError("Invalid UTF-8 encoding in headers".into()),
        };

        let mut lines = header_str.lines();
        let request_line = match lines.next() {
            Some(line) if !line.trim().is_empty() => line.trim(),
            _ => return ParseStatus::FatalError("Empty request line".into()),
        };

        let mut parts = request_line.split_whitespace();
        let method_str = match parts.next() {
            Some(m) => m,
            None => return ParseStatus::FatalError("Missing HTTP method".into()),
        };
        let uri = match parts.next() {
            Some(u) => u,
            None => return ParseStatus::FatalError("Missing URI".into()),
        };
        let version = parts.next().unwrap_or("HTTP/1.1");

        let method = HttpMethod::from_str_bytes(method_str);

        let mut headers = HttpHeaders::new();
        for line in lines {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }
            if let Some(colon_idx) = line.find(':') {
                let name = &line[..colon_idx];
                let val = &line[colon_idx + 1..];
                headers.insert(name, val);
            }
        }

        let body_start = header_end + delim_len;
        let content_length = headers.content_length().unwrap_or(0);
        let total_consumed = body_start + content_length;

        // If the body hasn't fully arrived yet, wait for more socket data
        if buf.len() < total_consumed {
            return ParseStatus::NeedMoreData;
        }

        let close_connection = headers.is_connection_close();

        // RFC 7230 §5.4: HTTP/1.1 requests MUST include a non-empty Host header
        if version == "HTTP/1.1" && !headers.has_valid_host() {
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

        let request = HttpRequest::new(method, uri.to_string(), version.to_string(), headers, body);

        ParseStatus::Complete(request, total_consumed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_get() {
        let wire = b"GET /add?a=2&b=3 HTTP/1.1\r\nHost: localhost:8080\r\n\r\n";
        match HttpParser::parse(wire) {
            ParseStatus::Complete(req, consumed) => {
                assert_eq!(consumed, wire.len());
                assert_eq!(req.method, HttpMethod::Get);
                assert_eq!(req.path, "/add");
                assert_eq!(req.get_query("a"), Some("2"));
                assert_eq!(req.get_query("b"), Some("3"));
            }
            other => panic!("Expected Complete, got {:?}", other),
        }
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
            other => panic!("Expected RecoverableError, got {:?}", other),
        }
    }

    #[test]
    fn test_pipelining_buffer_slicing() {
        let req1 = b"GET /add?a=2&b=3 HTTP/1.1\r\nHost: localhost:8080\r\n\r\n";
        let req2 = b"GET /sub?a=10&b=4 HTTP/1.1\r\nHost: localhost:8080\r\n\r\n";
        let mut combined = Vec::new();
        combined.extend_from_slice(req1);
        combined.extend_from_slice(req2);

        // First parse
        let (first_req, consumed1) = match HttpParser::parse(&combined) {
            ParseStatus::Complete(req, c) => (req, c),
            other => panic!("Expected Complete for req1, got {:?}", other),
        };
        assert_eq!(first_req.path, "/add");
        assert_eq!(consumed1, req1.len());

        // Drain first request
        combined.drain(..consumed1);

        // Second parse from remaining buffer
        let (second_req, consumed2) = match HttpParser::parse(&combined) {
            ParseStatus::Complete(req, c) => (req, c),
            other => panic!("Expected Complete for req2, got {:?}", other),
        };
        assert_eq!(second_req.path, "/sub");
        assert_eq!(consumed2, req2.len());
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
        // Append next request byte to test boundary protection
        wire.extend_from_slice(b"G");

        match HttpParser::parse(&wire) {
            ParseStatus::Complete(req, consumed) => {
                assert_eq!(req.body, body);
                assert_eq!(consumed, wire.len() - 1); // Byte 'G' was untouched!
            }
            other => panic!("Expected Complete, got {:?}", other),
        }
    }

    #[test]
    fn test_post_body_followed_by_get() {
        let post_req = b"POST /add HTTP/1.1\r\nHost: localhost\r\nContent-Length: 5\r\n\r\nHELLO";
        let get_req = b"GET /add?a=2&b=3 HTTP/1.1\r\nHost: localhost\r\n\r\n";
        let mut combined = Vec::new();
        combined.extend_from_slice(post_req);
        combined.extend_from_slice(get_req);

        let (req1, consumed1) = match HttpParser::parse(&combined) {
            ParseStatus::Complete(r, c) => (r, c),
            other => panic!("Expected Complete for POST, got {:?}", other),
        };
        assert_eq!(req1.method, HttpMethod::Post);
        assert_eq!(req1.body, b"HELLO");
        assert_eq!(consumed1, post_req.len());

        combined.drain(..consumed1);

        let (req2, consumed2) = match HttpParser::parse(&combined) {
            ParseStatus::Complete(r, c) => (r, c),
            other => panic!("Expected Complete for GET, got {:?}", other),
        };
        assert_eq!(req2.method, HttpMethod::Get);
        assert_eq!(req2.path, "/add");
        assert_eq!(consumed2, get_req.len());
    }
}
