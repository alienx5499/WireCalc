use super::status::HttpStatus;

/// Represents an HTTP/1.1 response with deterministic wire serialization.
#[derive(Debug, Clone)]
pub struct HttpResponse {
    pub status: HttpStatus,
    pub headers: Vec<(String, String)>,
    pub body: Vec<u8>,
    pub close_connection: bool,
}

impl HttpResponse {
    pub fn new(status: HttpStatus, body: Vec<u8>, close_connection: bool) -> Self {
        let mut resp = Self {
            status,
            headers: Vec::with_capacity(4),
            body,
            close_connection,
        };
        resp.add_header("Content-Length", &resp.body.len().to_string());
        resp.add_header("Content-Type", "text/plain; charset=utf-8");
        if close_connection {
            resp.add_header("Connection", "close");
        } else {
            resp.add_header("Connection", "keep-alive");
        }
        resp
    }

    pub fn ok(body_str: &str, close_connection: bool) -> Self {
        Self::new(
            HttpStatus::Ok,
            body_str.as_bytes().to_vec(),
            close_connection,
        )
    }

    pub fn bad_request(msg: &str, close_connection: bool) -> Self {
        let body = format!("Bad Request: {}\n", msg).into_bytes();
        Self::new(HttpStatus::BadRequest, body, close_connection)
    }

    pub fn not_found(msg: &str, close_connection: bool) -> Self {
        let body = format!("Not Found: {}\n", msg).into_bytes();
        Self::new(HttpStatus::NotFound, body, close_connection)
    }

    pub fn method_not_allowed(close_connection: bool) -> Self {
        let body = b"Method Not Allowed\n".to_vec();
        Self::new(HttpStatus::MethodNotAllowed, body, close_connection)
    }

    pub fn add_header(&mut self, name: &str, value: &str) {
        self.headers.push((name.to_string(), value.to_string()));
    }

    /// Serialize the response into wire bytes for direct transmission.
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(128 + self.body.len());
        out.extend_from_slice(b"HTTP/1.1 ");
        out.extend_from_slice(self.status.code_str().as_bytes());
        out.extend_from_slice(b" ");
        out.extend_from_slice(self.status.reason_phrase().as_bytes());
        out.extend_from_slice(b"\r\n");

        for (k, v) in &self.headers {
            out.extend_from_slice(k.as_bytes());
            out.extend_from_slice(b": ");
            out.extend_from_slice(v.as_bytes());
            out.extend_from_slice(b"\r\n");
        }
        out.extend_from_slice(b"\r\n");
        out.extend_from_slice(&self.body);
        out
    }
}
