//! Example demonstrating the #[rpc] blessed preset.
//!
//! `#[rpc]` = `#[jsonrpc]` + `#[openrpc]` + `#[serve(jsonrpc)]` in one attribute.
//!
//! ```bash
//! cargo run --example rpc_preset
//! # Then:
//! curl -X POST http://localhost:3000/rpc \
//!   -H 'Content-Type: application/json' \
//!   -d '{"jsonrpc":"2.0","method":"add","params":{"a":2,"b":3},"id":1}'
//! ```

use server_less::rpc;

#[derive(Clone)]
pub struct Calculator;

#[rpc]
impl Calculator {
    /// Add two numbers
    pub fn add(&self, a: i32, b: i32) -> i32 {
        a + b
    }

    /// Multiply two numbers
    pub fn multiply(&self, a: i32, b: i32) -> i32 {
        a * b
    }

    /// Integer division
    pub fn divide(&self, a: i32, b: i32) -> Result<i32, String> {
        if b == 0 {
            Err("division by zero".into())
        } else {
            Ok(a / b)
        }
    }
}

#[tokio::main]
async fn main() {
    // Print OpenRPC spec
    println!("OpenRPC spec:");
    println!(
        "{}",
        serde_json::to_string_pretty(&Calculator::openrpc_spec()).unwrap()
    );

    println!("\nStarting JSON-RPC server on http://localhost:3000");
    println!("  POST /rpc - JSON-RPC 2.0 endpoint");
    println!("  GET  /health - health check");
    Calculator.serve("0.0.0.0:3000").await.unwrap();
}
