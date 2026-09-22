use crate::calc::{CalcError, OperationRegistry, format_result};
use crate::http::{HttpMethod, HttpRequest, HttpResponse};
use std::sync::Arc;

/// Directs incoming HTTP requests to corresponding calculator operations.
#[derive(Clone)]
pub struct Router {
    registry: Arc<OperationRegistry>,
}

impl Router {
    pub fn new(registry: Arc<OperationRegistry>) -> Self {
        Self { registry }
    }

    /// Handles an incoming request and generates an appropriate HTTP response.
    pub fn route(&self, req: &HttpRequest) -> HttpResponse {
        let should_close = req.should_close();

        // Extract clean operation path: e.g. "/add" -> "add"
        let raw_path = req.path().trim();
        let path = raw_path.strip_prefix('/').unwrap_or(raw_path);

        // Check if operation exists in registry
        let op = match self.registry.get(path) {
            Some(operation) => operation,
            None => {
                // Unknown endpoint: 404 Not Found
                return HttpResponse::not_found("Unknown operation", should_close);
            }
        };

        // If operation exists, check allowed HTTP method (only GET is supported for calculation)
        if req.method != HttpMethod::Get {
            return HttpResponse::method_not_allowed(should_close);
        }

        // Parse query parameter 'a'
        let a_str = match req.get_query("a") {
            Some(v) => v,
            None => {
                return HttpResponse::bad_request("Missing parameter 'a'", should_close);
            }
        };
        let a = match a_str.parse::<f64>() {
            Ok(val) => val,
            Err(_) => {
                return HttpResponse::bad_request(
                    &format!("Parameter 'a' is not a valid number: '{}'", a_str),
                    should_close,
                );
            }
        };

        // Parse query parameter 'b'
        let b_str = match req.get_query("b") {
            Some(v) => v,
            None => {
                return HttpResponse::bad_request("Missing parameter 'b'", should_close);
            }
        };
        let b = match b_str.parse::<f64>() {
            Ok(val) => val,
            Err(_) => {
                return HttpResponse::bad_request(
                    &format!("Parameter 'b' is not a valid number: '{}'", b_str),
                    should_close,
                );
            }
        };

        // Execute operation
        match op.execute(a, b) {
            Ok(result) => {
                let formatted = format_result(result);
                HttpResponse::ok(&formatted, should_close)
            }
            Err(CalcError::DivisionByZero) => {
                HttpResponse::bad_request("Division by zero", should_close)
            }
            Err(e) => HttpResponse::bad_request(&e.to_string(), should_close),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::calc::{AddOperation, DivOperation, MulOperation, SubOperation};
    use crate::http::{HttpHeaders, HttpStatus, HttpVersion};

    fn setup_router() -> Router {
        let mut registry = OperationRegistry::new();
        registry.register(AddOperation);
        registry.register(SubOperation);
        registry.register(MulOperation);
        registry.register(DivOperation);
        Router::new(Arc::new(registry))
    }

    fn make_request(method: HttpMethod, uri: &str) -> HttpRequest {
        let mut headers = HttpHeaders::new();
        headers.insert("Host", "localhost:8080");
        HttpRequest::new(
            method,
            uri.to_string(),
            HttpVersion::Http11,
            headers,
            Vec::new(),
        )
    }

    #[test]
    fn test_valid_calculations() {
        let router = setup_router();

        let resp_add = router.route(&make_request(HttpMethod::Get, "/add?a=2&b=3"));
        assert_eq!(resp_add.status, HttpStatus::Ok);
        assert_eq!(resp_add.body, b"5\n");

        let resp_sub = router.route(&make_request(HttpMethod::Get, "/sub?a=10&b=4"));
        assert_eq!(resp_sub.status, HttpStatus::Ok);
        assert_eq!(resp_sub.body, b"6\n");

        let resp_mul = router.route(&make_request(HttpMethod::Get, "/mul?a=6&b=7"));
        assert_eq!(resp_mul.status, HttpStatus::Ok);
        assert_eq!(resp_mul.body, b"42\n");

        let resp_div = router.route(&make_request(HttpMethod::Get, "/div?a=9&b=3"));
        assert_eq!(resp_div.status, HttpStatus::Ok);
        assert_eq!(resp_div.body, b"3\n");
    }

    #[test]
    fn test_division_by_zero() {
        let router = setup_router();
        let resp = router.route(&make_request(HttpMethod::Get, "/div?a=1&b=0"));
        assert_eq!(resp.status, HttpStatus::BadRequest);
    }

    #[test]
    fn test_invalid_param() {
        let router = setup_router();
        let resp = router.route(&make_request(HttpMethod::Get, "/add?a=x&b=3"));
        assert_eq!(resp.status, HttpStatus::BadRequest);
    }

    #[test]
    fn test_not_found() {
        let router = setup_router();
        let resp = router.route(&make_request(HttpMethod::Get, "/pow?a=2&b=8"));
        assert_eq!(resp.status, HttpStatus::NotFound);
    }

    #[test]
    fn test_method_not_allowed() {
        let router = setup_router();
        let resp = router.route(&make_request(HttpMethod::Post, "/add"));
        assert_eq!(resp.status, HttpStatus::MethodNotAllowed);
    }
}
