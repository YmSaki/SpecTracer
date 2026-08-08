/// Add two integers.
pub fn add(left: i32, right: i32) -> i32 {
    left + right
}

/// Evaluate a simple operation.
pub fn evaluate(left: i32, right: i32, op: char) -> Result<i32, &'static str> {
    match op {
        '+' => Ok(add(left, right)),
        '-' => Ok(left - right),
        '*' => Ok(left * right),
        _ => Err("unsupported operation"),
    }
}

