/// FFI backend for JavaScript code generation
/// Currently only Koffi is supported, but this abstraction allows for future backends
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum FfiBackend {
    /// Koffi FFI backend
    #[default]
    Koffi,
}
