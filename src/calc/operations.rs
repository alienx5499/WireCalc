use super::error::CalcError;
use super::operation::Operation;

pub struct AddOperation;
impl Operation for AddOperation {
    fn name(&self) -> &str {
        "add"
    }

    fn execute(&self, a: f64, b: f64) -> Result<f64, CalcError> {
        Ok(a + b)
    }
}

pub struct SubOperation;
impl Operation for SubOperation {
    fn name(&self) -> &str {
        "sub"
    }

    fn execute(&self, a: f64, b: f64) -> Result<f64, CalcError> {
        Ok(a - b)
    }
}

pub struct MulOperation;
impl Operation for MulOperation {
    fn name(&self) -> &str {
        "mul"
    }

    fn execute(&self, a: f64, b: f64) -> Result<f64, CalcError> {
        Ok(a * b)
    }
}

pub struct DivOperation;
impl Operation for DivOperation {
    fn name(&self) -> &str {
        "div"
    }

    fn execute(&self, a: f64, b: f64) -> Result<f64, CalcError> {
        if b == 0.0 || b.abs() < 1e-15 {
            return Err(CalcError::DivisionByZero);
        }
        Ok(a / b)
    }
}

/// Formats a calculation result cleanly: integers as whole numbers, floats with decimals.
pub fn format_result(val: f64) -> String {
    if (val.fract()).abs() < 1e-12 {
        format!("{}\n", val as i64)
    } else {
        format!("{}\n", val)
    }
}
