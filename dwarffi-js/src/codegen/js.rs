/// JavaScript code generation dispatch
use anyhow::Result;
use dwarffi::{FunctionSignature, TypeRegistry};

use super::backend::FfiBackend;
use super::koffi;

pub struct JsCodegen;

impl JsCodegen {
    pub fn generate_module(
        type_registry: &TypeRegistry,
        functions: &[FunctionSignature],
        generate_types: bool,
        generate_functions: bool,
        library_path: &str,
        _backend: FfiBackend,
    ) -> Result<String> {
        // Currently only Koffi is supported
        koffi::generate(
            type_registry,
            functions,
            generate_types,
            generate_functions,
            library_path,
        )
    }
}
