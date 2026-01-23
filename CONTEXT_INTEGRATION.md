# Context Integration - Developer Guide

This document shows how the shared Context helpers make it easy to add Context injection to any protocol macro.

## Shared Helpers (`context.rs`)

All protocol macros can use these helpers:

```rust
use crate::context::{
    has_qualified_context,           // First pass: detect qualified Context usage
    partition_context_params,         // Separate Context from regular params
    should_inject_context,            // Check if a type should be injected
    generate_http_context_extraction, // HTTP-specific extraction
    generate_cli_context_extraction,  // CLI-specific extraction
};
```

## Integration Pattern

### Step 1: Two-Pass Detection

```rust
pub(crate) fn expand_YOUR_PROTOCOL(args: YourArgs, impl_block: ItemImpl) -> syn::Result<TokenStream2> {
    let struct_name = get_impl_name(&impl_block)?;
    let methods = extract_methods(&impl_block)?;

    // PASS 1: Scan for qualified server_less::Context usage
    let has_qualified = has_qualified_context(&methods);

    // ... rest of your macro
}
```

### Step 2: Parameter Partitioning

```rust
fn generate_handler(method: &MethodInfo, has_qualified: bool) -> syn::Result<TokenStream2> {
    // Separate Context params from regular params
    let (context_param, regular_params) = partition_context_params(&method.params, has_qualified)?;

    let mut extractions = Vec::new();
    let mut calls = Vec::new();

    // Generate Context extraction if needed
    if context_param.is_some() {
        let (extraction, call) = generate_YOUR_PROTOCOL_context_extraction();
        extractions.push(extraction);
        calls.push(call);
    }

    // Handle regular params...
    for param in regular_params {
        // Your existing parameter handling
    }
}
```

### Step 3: Protocol-Specific Extraction

Add a helper to `context.rs` for your protocol:

```rust
/// Generate Context extraction code for YOUR_PROTOCOL
pub fn generate_YOUR_PROTOCOL_context_extraction() -> (TokenStream2, TokenStream2) {
    let extraction = quote! {
        // Your protocol's extractor (e.g., headers, metadata, etc.)
        __context_source: YourExtractorType
    };

    let call = quote! {
        {
            let mut __ctx = ::server_less::Context::new();
            // Populate from your protocol's data
            __ctx
        }
    };

    (extraction, call)
}
```

## Example: WebSocket Integration

Here's how to add Context to WebSocket (not yet implemented):

```rust
// In ws.rs
use crate::context::{
    has_qualified_context,
    partition_context_params,
    generate_http_context_extraction, // WebSocket uses HTTP upgrade, so same headers
};

pub(crate) fn expand_ws(args: WsArgs, impl_block: ItemImpl) -> syn::Result<TokenStream2> {
    let struct_name = get_impl_name(&impl_block)?;
    let methods = extract_methods(&impl_block)?;

    // PASS 1: Detect qualified Context
    let has_qualified = has_qualified_context(&methods);

    // ... generate handlers with has_qualified
}

fn generate_ws_handler(method: &MethodInfo, has_qualified: bool) -> syn::Result<TokenStream2> {
    // Separate Context from regular params
    let (context_param, regular_params) = partition_context_params(&method.params, has_qualified)?;

    if context_param.is_some() {
        // WebSocket upgrades from HTTP, so we can access upgrade headers
        let (extraction, call) = generate_http_context_extraction();
        // Add to handler signature and method call
    }

    // ... rest of WebSocket handler generation
}
```

## Example: CLI Integration

Here's how to add Context to CLI (not yet implemented):

```rust
// In cli.rs
use crate::context::{
    has_qualified_context,
    partition_context_params,
    generate_cli_context_extraction,
};

pub(crate) fn expand_cli(args: CliArgs, impl_block: ItemImpl) -> syn::Result<TokenStream2> {
    let struct_name = get_impl_name(&impl_block)?;
    let methods = extract_methods(&impl_block)?;

    // PASS 1: Detect qualified Context
    let has_qualified = has_qualified_context(&methods);

    // ... generate CLI handlers
}

fn generate_cli_handler(method: &MethodInfo, has_qualified: bool) -> syn::Result<TokenStream2> {
    let (context_param, regular_params) = partition_context_params(&method.params, has_qualified)?;

    if context_param.is_some() {
        // CLI gets context from environment variables
        let (extraction, call) = generate_cli_context_extraction();
        // Add to handler logic
    }

    // ... rest of CLI handler generation
}
```

## Benefits of This Approach

1. **No Code Duplication**: Detection logic is shared across all protocols
2. **Consistent Behavior**: All protocols handle Context collision the same way
3. **Easy Extension**: Adding Context to a new protocol is ~10 lines of code
4. **Type Safety**: Compile errors if you forget to pass `has_qualified`
5. **Testable**: Core logic is tested once in `context.rs`

## Testing

Each protocol should test:
- Basic Context injection works
- Qualified vs bare Context collision detection
- Context excluded from protocol specs (OpenAPI, OpenRPC, etc.)
- Methods without Context still work

See `context_tests.rs` for HTTP examples.
