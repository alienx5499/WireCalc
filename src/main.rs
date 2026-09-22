mod calc;
mod http;
mod server;

use std::sync::Arc;
use std::time::Duration;

use calc::{AddOperation, DivOperation, MulOperation, OperationRegistry, SubOperation};
use server::{Router, TcpServer};

fn main() -> std::io::Result<()> {
    // 1. Initialize operation registry
    let mut registry = OperationRegistry::new();
    registry.register(AddOperation);
    registry.register(SubOperation);
    registry.register(MulOperation);
    registry.register(DivOperation);

    // 2. Initialize request router
    let router = Router::new(Arc::new(registry));

    // 3. Configure and launch TCP socket server on port 8080
    // Idle timeout set to 15 seconds to allow persistent connection testing
    let server = TcpServer::new("0.0.0.0:8080", router, Duration::from_secs(15));

    server.run()
}
