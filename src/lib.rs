//! ffitool - extract function signatures from C libraries using DWARF debug
//! information
//!
//! - only works for libraries compiled with DWARF info (e.g. gcc -g ...)
//! - only works on macOS and Linux
//! - some limitations around arrays and nested types
//! - use at your own risk!

mod dwarf_analyzer;
mod symbol_reader;
mod type_resolver;
pub mod types;

pub use dwarf_analyzer::DwarfAnalyzer;
pub use types::{FunctionSignature, Parameter};
