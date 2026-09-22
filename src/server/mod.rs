pub mod handler;
pub mod router;

use std::io;
use std::net::TcpListener;
use std::sync::Arc;
use std::thread;
use std::time::Duration;

pub use handler::ConnectionHandler;
pub use router::Router;

/// Multi-threaded TCP server that binds to a socket and delegates each connection
/// to ConnectionHandler.
pub struct TcpServer {
    addr: String,
    handler: Arc<ConnectionHandler>,
}

impl TcpServer {
    pub fn new(addr: impl Into<String>, router: Router, idle_timeout: Duration) -> Self {
        let handler = Arc::new(ConnectionHandler::new(Arc::new(router), idle_timeout));
        Self {
            addr: addr.into(),
            handler,
        }
    }

    /// Starts the blocking listener loop, spawning a worker thread per incoming socket.
    pub fn run(&self) -> io::Result<()> {
        let listener = TcpListener::bind(&self.addr)?;
        println!(
            "Wirecalc HTTP/1.1 Persistent Server listening on {}",
            self.addr
        );

        for stream in listener.incoming() {
            match stream {
                Ok(stream) => {
                    let handler = Arc::clone(&self.handler);
                    thread::spawn(move || {
                        if let Err(err) = handler.handle(stream) {
                            // Non-fatal per-connection error (e.g., client abrupt disconnect)
                            let _ = err;
                        }
                    });
                }
                Err(e) => {
                    eprintln!("Warning: Failed to accept connection: {}", e);
                }
            }
        }

        Ok(())
    }
}
