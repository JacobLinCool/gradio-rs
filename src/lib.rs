//! Rust client library for Gradio apps and Hugging Face Spaces.
//!
//! The primary error model is [`Error`] and [`Result<T>`].
//! `anyhow` is still re-exported temporarily for downstream compatibility,
//! but new code should prefer `gradio::Error` and `gradio::Result<T>`.
//!
pub mod client;
pub mod constants;
pub mod data;
pub mod error;
pub mod space;
pub mod stream;
pub mod structs;
pub mod sync;

pub use client::*;
pub use data::*;
pub use error::*;
pub use stream::*;

// Re-export anyhow for downstream compatibility during the error-model transition.
pub use anyhow;
pub use serde;
pub use tokio;
