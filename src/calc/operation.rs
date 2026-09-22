use super::error::CalcError;
use std::sync::Arc;

/// Core interface for any arithmetic operation.
pub trait Operation: Send + Sync {
    fn name(&self) -> &'static str;
    fn execute(&self, a: f64, b: f64) -> Result<f64, CalcError>;
}

/// Registry that manages arithmetic operations for request dispatching.
/// Uses a compact contiguous slice for sub-nanosecond L1 cache lookups without hashing overhead.
#[derive(Default, Clone)]
pub struct OperationRegistry {
    operations: Vec<(&'static str, Arc<dyn Operation>)>,
}

impl OperationRegistry {
    pub fn new() -> Self {
        Self {
            operations: Vec::with_capacity(4),
        }
    }

    /// Register a new operation conforming to the Operation trait without string allocation.
    pub fn register<O: Operation + 'static>(&mut self, op: O) {
        self.operations.push((op.name(), Arc::new(op)));
    }

    /// Retrieve an operation by name with cache-friendly linear scan over registered operations.
    pub fn get(&self, name: &str) -> Option<Arc<dyn Operation>> {
        self.operations
            .iter()
            .find(|(k, _)| *k == name)
            .map(|(_, op)| Arc::clone(op))
    }
}
