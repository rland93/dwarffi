/// dispatch code-gen either to koffi or ref-napi backend
use anyhow::Result;
use dwarffi::{FunctionSignature, TypeRegistry};

use super::backend::FfiBackend;
use super::{koffi, ref_napi};

pub struct JsCodegen;

impl JsCodegen {
    pub fn generate_module(
        type_registry: &TypeRegistry,
        functions: &[FunctionSignature],
        generate_types: bool,
        generate_functions: bool,
        library_path: &str,
        backend: FfiBackend,
    ) -> Result<String> {
        match backend {
            FfiBackend::Koffi => koffi::generate(
                type_registry,
                functions,
                generate_types,
                generate_functions,
                library_path,
            ),
            FfiBackend::RefNapi => ref_napi::generate(
                type_registry,
                functions,
                generate_types,
                generate_functions,
                library_path,
            ),
        }
    }
}
