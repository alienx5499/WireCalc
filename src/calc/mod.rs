#![allow(unused_imports)]
pub mod error;
pub mod operation;
pub mod operations;

pub use error::CalcError;
pub use operation::{Operation, OperationRegistry};
pub use operations::{AddOperation, DivOperation, MulOperation, SubOperation, format_result};
