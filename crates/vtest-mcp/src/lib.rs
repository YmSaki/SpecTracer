//! MCP transport adapter boundary (M9).

use serde_json::Value;

/// M9 will route every tool through the same application functions as the
/// CLI.  Keeping this adapter empty in S0 prevents a second decision engine.
pub fn not_implemented() -> Value {
    serde_json::json!({
        "ok": false,
        "data": null,
        "diagnostics": [{
            "code": "E-CORE-001",
            "severity": "error",
            "message": "MCP server is not implemented before milestone M9"
        }]
    })
}
