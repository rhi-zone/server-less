//! Example: Composing OpenAPI specs from multiple services with OpenApiBuilder.
//!
//! Shows how to merge specs from independent services into a single API spec,
//! including schema deduplication and conflict detection.
//!
//! Run: cargo run --example openapi_composition

use server_less::{OpenApiBuilder, http};

// ============================================================================
// Service A: User management
// ============================================================================

#[derive(Clone)]
struct UserService;

#[http(prefix = "/api/users")]
impl UserService {
    /// List all users
    pub fn list_users(&self) -> Vec<String> {
        vec![]
    }

    /// Get user by ID
    pub fn get_user(&self, _user_id: String) -> Option<String> {
        None
    }

    /// Create a user
    pub fn create_user(&self, name: String, email: String) -> String {
        format!("{}:{}", name, email)
    }
}

// ============================================================================
// Service B: Order management
// ============================================================================

#[derive(Clone)]
struct OrderService;

#[http(prefix = "/api/orders")]
impl OrderService {
    /// List all orders
    pub fn list_orders(&self) -> Vec<String> {
        vec![]
    }

    /// Get order by ID
    pub fn get_order(&self, _order_id: String) -> Option<String> {
        None
    }

    /// Create an order
    pub fn create_order(&self, product: String, quantity: u32) -> String {
        format!("{}x{}", product, quantity)
    }
}

fn main() {
    // Each service generates its own OpenAPI spec
    let user_spec = UserService::openapi_spec();
    let order_spec = OrderService::openapi_spec();

    println!("=== User Service Spec ===");
    println!("{}", serde_json::to_string_pretty(&user_spec).unwrap());

    println!("\n=== Order Service Spec ===");
    println!("{}", serde_json::to_string_pretty(&order_spec).unwrap());

    // Compose into a single spec with OpenApiBuilder
    let combined = OpenApiBuilder::new()
        .title("My Store API")
        .version("1.0.0")
        .description("Combined API from UserService and OrderService")
        .merge(user_spec)
        .expect("merge user spec")
        .merge(order_spec)
        .expect("merge order spec")
        .build();

    println!("\n=== Combined Spec ===");
    println!("{}", serde_json::to_string_pretty(&combined).unwrap());

    // Verify the combined spec has paths from both services
    let paths = combined["paths"].as_object().unwrap();
    println!("\nEndpoints in combined spec:");
    for (path, methods) in paths {
        let method_list: Vec<_> = methods.as_object().unwrap().keys().collect();
        println!(
            "  {} [{}]",
            path,
            method_list
                .iter()
                .map(|m| m.to_uppercase())
                .collect::<Vec<_>>()
                .join(", ")
        );
    }

    // You can also compose using typed paths (merge_paths)
    let typed_combined = OpenApiBuilder::new()
        .title("My Store API (typed)")
        .version("1.0.0")
        .merge_paths(UserService::http_openapi_paths())
        .merge_paths(OrderService::http_openapi_paths())
        .build();

    let typed_paths = typed_combined["paths"].as_object().unwrap();
    println!("\nEndpoints via typed merge:");
    for (path, methods) in typed_paths {
        let method_list: Vec<_> = methods.as_object().unwrap().keys().collect();
        println!(
            "  {} [{}]",
            path,
            method_list
                .iter()
                .map(|m| m.to_uppercase())
                .collect::<Vec<_>>()
                .join(", ")
        );
    }
}
