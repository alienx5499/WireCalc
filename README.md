# Wirecalc: Persistent HTTP/1.1 Calculator Socket Engine

> Built in pure Rust (Standard Library only - zero third-party web crates or frameworks).
> Implements persistent keep-alive connections, streaming socket framing, and pipelining.

---

## 1. Overview & Assignment Specification

In HTTP/1.0, requests ended when the server closed the TCP socket (`at EOF`). In **HTTP/1.1**, connections are **persistent by default** (`keep-alive`). This presents a fundamental low-level framing challenge:
> *"Where does this request end and the next one begin? Byte $n+1$ belongs to somebody else."*

`Wirecalc` solves this using raw socket I/O (`std::net::TcpListener` and `std::net::TcpStream`), exact byte-level boundary tracking with `Content-Length`, and FIFO pipelining support.

### Supported Features & Verification Criteria

| Request | Expected Status | Body / Action |
| :--- | :---: | :--- |
| `GET /add?a=2&b=3` | **`200 OK`** | `5\n` |
| `GET /sub?a=10&b=4` | **`200 OK`** | `6\n` |
| `GET /mul?a=6&b=7` | **`200 OK`** | `42\n` |
| `GET /div?a=9&b=3` | **`200 OK`** | `3\n` |
| `GET /div?a=1&b=0` | **`400 Bad Request`** | Division by zero check |
| `GET /add?a=x&b=3` | **`400 Bad Request`** | Non-numeric parameter validation |
| `GET /pow?a=2&b=8` | **`404 Not Found`** | Unknown route |
| `POST /add` | **`405 Method Not Allowed`** | Method validation (only `GET` for calculation) |
| `GET /add` (no `Host`) | **`400 Bad Request`** | Strict RFC 7230 §5.4 compliance |
| **All 6 requests on 1 socket** | **Sequential** | `socket still open: True` |

### Stretch Goals Implemented
- **Full HTTP Pipelining:** Batches of multiple requests sent in a single `sendall()` write are consumed and replied to in strict FIFO order.
- **`Connection: close` Handling:** Honors explicit client connection termination without leaking sockets.
- **Configurable Idle Timeout:** Read timeout on `TcpStream` prevents hanging socket descriptors.

---

## 2. Architecture & Module Overview

Overview of the core modules and responsibilities:

| Module | Responsibility |
| :--- | :--- |
| [`src/calc/operation.rs`](src/calc/operation.rs) | Core `Operation` trait and extensible `OperationRegistry`. |
| [`src/calc/operations.rs`](src/calc/operations.rs) | Implementations of `add`, `sub`, `mul`, and `div` with zero-division protection. |
| [`src/calc/error.rs`](src/calc/error.rs) | Domain error definitions for arithmetic evaluation. |
| [`src/http/parser.rs`](src/http/parser.rs) | Wire-level byte scanning, delimiter extraction, boundary isolation, and RFC Host checks. |
| [`src/http/headers.rs`](src/http/headers.rs) | Case-insensitive ASCII header lookup and protocol flags. |
| [`src/http/request.rs`](src/http/request.rs) | Request data model with single-pass zero-allocation URI parameter parser. |
| [`src/http/response.rs`](src/http/response.rs) | Wire byte serialization with deterministic `Content-Length`. |
| [`src/http/status.rs`](src/http/status.rs) | HTTP status codes, reason phrases, and wire formatting helpers. |
| [`src/http/method.rs`](src/http/method.rs) | Supported HTTP verbs and zero-allocation method parser. |
| [`src/http/version.rs`](src/http/version.rs) | HTTP protocol version definitions and zero-copy parser. |
| [`src/server/router.rs`](src/server/router.rs) | Request router mapping paths and methods to operations. |
| [`src/server/handler.rs`](src/server/handler.rs) | Socket connection event loop, idle timeouts, buffer draining, and FIFO pipelined dispatch. |
| [`src/server/mod.rs`](src/server/mod.rs) | TCP socket listener and thread dispatching. |
| [`src/main.rs`](src/main.rs) | Server initialization and listener binding on `0.0.0.0:8080`. |

---

## 3. Algorithmic Optimizations & Framing Mechanics

### Stream Framing Architecture

<img width="5220" height="2010" alt="image" src="https://github.com/user-attachments/assets/ac26618c-82e0-40a0-a9cf-27c1f4f51c12" />

### Connection Lifecycle & Protocol Flow

<img width="6335" height="3520" alt="image" src="https://github.com/user-attachments/assets/ea926efb-343a-49c1-b2e4-a4749a70a714" />

### Key Performance Mechanics

1. **Sliding Window Buffer Draining (`Vec::drain`)**:
   - Rather than allocating fresh buffers per request, incoming bytes accumulate in a single connection buffer.
   - Once a request's header + `Content-Length` bytes are consumed, `buffer.drain(..consumed)` removes *exactly* those bytes in $O(M)$ time.
   - Any surplus bytes remaining in the buffer belong to pipelined requests and are immediately parsed without waiting for a new socket read.
2. **Fast Delimiter Search**:
   - Single-pass `\r\n\r\n` window scanner scans byte slices without UTF-8 string conversions until the header boundary is confirmed.
3. **Zero-Allocation Query String Slicing**:
   - Parameter extraction (`a=2&b=3`) performs zero-allocation linear scanning using borrowed `&str` slices.
4. **Clean Integer vs Float Formatting**:
   - Values like `5.0` are formatted as `5\n`, eliminating extraneous floating-point decimals when not required.

---

## 4. Building & Running

### Prerequisites
- Rust (`cargo`, `rustc`)

### Build and Run Server
```bash
# In Network Architecture/Wirecalc/
cargo run --release
```
The server binds to `0.0.0.0:8080`.

### Run Tests
```bash
cargo test
```
All built-in tests run natively in Rust:
- Query parsing & routing
- Host header enforcement (RFC 7230 §5.4)
- Content-Length exact boundary isolation
- Pipelining buffer slicing
- Division by zero and parameter validation

---

## 5. Verification & Socket Demonstration

### End-to-End Pipeline Verification
Demonstrates keeping a single TCP connection alive across multiple requests (1 handshake, 6 responses):

```bash
printf "GET /add?a=2&b=3 HTTP/1.1\r\nHost: localhost\r\n\r\n\
GET /sub?a=10&b=4 HTTP/1.1\r\nHost: localhost\r\n\r\n\
GET /mul?a=6&b=7 HTTP/1.1\r\nHost: localhost\r\n\r\n\
GET /div?a=1&b=0 HTTP/1.1\r\nHost: localhost\r\n\r\n\
GET /pow?a=2&b=8 HTTP/1.1\r\nHost: localhost\r\n\r\n\
POST /add HTTP/1.1\r\nHost: localhost\r\nContent-Length: 0\r\n\r\n" | nc localhost 8080
```

### Interactive Protocol Inspection
For manual inspection over raw TCP:

```bash
nc -C localhost 8080
```

```http
GET /add?a=2&b=3 HTTP/1.1
Host: localhost

HTTP/1.1 200 OK
Content-Length: 2
Content-Type: text/plain; charset=utf-8
Connection: keep-alive

5
```

*(Note: Per RFC 7230 §3, headers are terminated by a blank line `\r\n\r\n`.)*

