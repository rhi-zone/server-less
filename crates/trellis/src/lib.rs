//! Trellis - Composable derive macros for Rust
//!
//! A collection of derive macros for common patterns like server setup,
//! configuration loading, and more. Designed for composition via attributes.

/// Placeholder for future macro implementations
pub fn add(left: u64, right: u64) -> u64 {
    left + right
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        let result = add(2, 2);
        assert_eq!(result, 4);
    }
}
