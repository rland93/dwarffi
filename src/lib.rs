//! ffitool - extract function signatures from C libraries using DWARF debug
//! information
//!
//! - only works for libraries compiled with DWARF info (e.g. gcc -g ...)
//! - only works on macOS and Linux
//! - some limitations around arrays and nested types
//! - use at your own risk!

mod dwarf_analyzer;
mod reader;
mod symbol_reader;
pub mod type_registry;
mod type_resolver;
pub mod types;

pub use dwarf_analyzer::DwarfAnalyzer;
pub use type_registry::{
    BaseTypeKind, EnumVariant, StructField, Type, TypeId, TypeRegistry, UnionField,
};
pub use types::{FunctionSignature, Parameter};
