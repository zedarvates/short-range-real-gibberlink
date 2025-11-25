//! WebAssembly-compatible version of RealGibber core
//!
//! This is a stripped-down version that includes only the functionality
//! needed for WebAssembly builds, excluding async/tokio dependencies.

pub mod crypto;
pub mod visual;
pub mod wasm;

// Re-export for convenience
pub use crypto::CryptoEngine;
pub use visual::{VisualEngine, VisualPayload};
pub use wasm::*;