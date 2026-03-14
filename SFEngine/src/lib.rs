pub mod db;
pub mod llm;
pub mod agents;
pub mod executor;
pub mod engine;
pub mod ideation;
pub mod tools;
pub mod protocols;
pub mod guard;
pub mod catalog;
pub mod bench;
pub mod sandbox;
pub mod indexer;
pub mod eval;
pub mod cache;
pub mod workers;
pub mod ops;
pub mod quality;
pub mod observability;
pub mod a2a;
pub mod mcp;
mod ffi;

pub use ffi::*;

// Ref: FT-SSF-020