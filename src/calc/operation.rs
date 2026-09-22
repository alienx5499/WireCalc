use super::error::CalcError;
use std::collections::HashMap;
use std::sync::Arc;

/// Core interface for any arithmetic operation.
pub trait Operation: Send + Sync {
    fn name(&self) -> &str;
    fn execute(&self, a: f64, b: f64) -> Result<f64, CalcError>;
}

/// Registry that manages arithmetic operations for request dispatching.
#[derive(Default, Clone)]
pub struct OperationRegistry {
    operations: HashMap<String, Arc<dyn Operation>>,
}

impl OperationRegistry {
    pub fn new() -> Self {
        Self {
            operations: HashMap::new(),
        }
    }

    /// Register a new operation conforming to the Operation trait.
    pub fn register<O: Operation + 'static>(&mut self, op: O) {
        self.operations.insert(op.name().to_string(), Arc::new(op));
    }

    /// Retrieve an operation by name.
    pub fn get(&self, name: &str) -> Option<Arc<dyn Operation>> {
        self.operations.get(name).cloned()
    }
}
