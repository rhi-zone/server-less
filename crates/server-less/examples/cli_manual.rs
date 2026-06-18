//! Example demonstrating the `--manual` whole-tree reference surface.
//!
//! `--manual` is a global flag on every `#[cli]` command. It emits the reference
//! document for the command **subtree rooted at the invoked node**, composing
//! with the existing format flags (`--json` / `--jsonl` / `--jq`).
//!
//! ```bash
//! # Whole tree, human-readable
//! cargo run --example cli_manual -- --manual
//!
//! # Whole tree as one path-keyed JSON document (each node: description,
//! # input_schema, output_schema)
//! cargo run --example cli_manual -- --manual --json
//!
//! # Subtree scoping: just the `posts` group (and its nested `comments`)
//! cargo run --example cli_manual -- posts --manual
//!
//! # A single leaf's entry
//! cargo run --example cli_manual -- health --manual
//! ```

use serde::{Deserialize, Serialize};
use server_less::cli;

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct Comment {
    pub id: String,
    pub body: String,
}

impl std::fmt::Display for Comment {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.id, self.body)
    }
}

#[derive(Clone, Default)]
pub struct CommentService;

#[cli(name = "comments", version = "0.1.0")]
impl CommentService {
    /// List comments on a post
    pub fn list(&self) -> Vec<Comment> {
        vec![Comment {
            id: "c1".to_string(),
            body: "first!".to_string(),
        }]
    }

    /// Add a comment to a post
    pub fn add(&self, body: String) -> Comment {
        Comment {
            id: "c2".to_string(),
            body,
        }
    }
}

#[derive(Clone, Default)]
pub struct PostService {
    comments: CommentService,
}

#[cli(name = "posts", version = "0.1.0")]
impl PostService {
    /// List all posts
    pub fn list(&self) -> Vec<String> {
        vec!["hello-world".to_string()]
    }

    /// Create a post with a title
    pub fn create(&self, title: String) -> String {
        format!("created: {title}")
    }

    /// Comment management (nested mount — depth 3 under the root)
    pub fn comments(&self) -> &CommentService {
        &self.comments
    }
}

#[derive(Clone, Default)]
pub struct BlogApp {
    posts: PostService,
}

#[cli(name = "blog", version = "0.1.0", description = "A tiny blog CLI")]
impl BlogApp {
    /// Check service health
    pub fn health(&self) -> String {
        "ok".to_string()
    }

    /// Post management commands
    pub fn posts(&self) -> &PostService {
        &self.posts
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let app = BlogApp::default();
    app.cli_run()
}
