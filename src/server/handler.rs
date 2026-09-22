use std::io::{self, Read, Write};
use std::net::TcpStream;
use std::sync::Arc;
use std::time::Duration;

use crate::http::{HttpParser, HttpResponse, ParseStatus};
use crate::server::router::Router;

/// Socket connection handler responsible for persistent connection lifecycle,
/// pipelining FIFO queue processing, and idle timeouts.
pub struct ConnectionHandler {
    router: Arc<Router>,
    idle_timeout: Duration,
}

impl ConnectionHandler {
    pub fn new(router: Arc<Router>, idle_timeout: Duration) -> Self {
        Self {
            router,
            idle_timeout,
        }
    }

    /// Handles an established TCP connection, servicing all persistent requests until close or timeout.
    pub fn handle(&self, mut stream: TcpStream) -> io::Result<()> {
        let _ = stream.set_read_timeout(Some(self.idle_timeout));

        let mut buffer: Vec<u8> = Vec::with_capacity(4096);
        let mut read_buf = [0u8; 4096];

        'connection: loop {
            // Process any fully buffered requests first (supporting pipelining)
            loop {
                match HttpParser::parse(&buffer) {
                    ParseStatus::Complete(req, consumed) => {
                        buffer.drain(..consumed);
                        let should_close = req.should_close();
                        let resp = self.router.route(&req);
                        let bytes = resp.to_bytes();

                        stream.write_all(&bytes)?;
                        stream.flush()?;

                        if should_close || resp.close_connection {
                            break 'connection;
                        }
                        // Continue parsing buffer in case more requests are pipelined
                    }
                    ParseStatus::RecoverableError {
                        error,
                        consumed_bytes,
                        close_connection,
                    } => {
                        buffer.drain(..consumed_bytes);
                        let resp = HttpResponse::bad_request(&error, close_connection);
                        let bytes = resp.to_bytes();

                        let _ = stream.write_all(&bytes);
                        let _ = stream.flush();

                        if close_connection {
                            break 'connection;
                        }
                    }
                    ParseStatus::FatalError(err) => {
                        let resp = HttpResponse::bad_request(&err, true);
                        let _ = stream.write_all(&resp.to_bytes());
                        let _ = stream.flush();
                        break 'connection;
                    }
                    ParseStatus::NeedMoreData => {
                        // Incomplete request; need to read more bytes from TCP stream
                        break;
                    }
                }
            }

            // Read the next chunk of bytes from the wire
            match stream.read(&mut read_buf) {
                Ok(0) => {
                    // Clean EOF: client closed the connection
                    break 'connection;
                }
                Ok(n) => {
                    buffer.extend_from_slice(&read_buf[..n]);
                }
                Err(ref e)
                    if e.kind() == io::ErrorKind::WouldBlock
                        || e.kind() == io::ErrorKind::TimedOut =>
                {
                    // Idle timeout expired without new request
                    break 'connection;
                }
                Err(ref e) if e.kind() == io::ErrorKind::Interrupted => {
                    continue;
                }
                Err(e) => {
                    return Err(e);
                }
            }
        }

        Ok(())
    }
}
