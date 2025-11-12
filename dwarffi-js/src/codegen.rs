/// Code generation module for creating FFI bindings from DWARF type information
pub mod backend;
pub mod js;
mod koffi;

pub use backend::FfiBackend;
pub use js::JsCodegen;
