#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FfiBackend {
    /// koffi
    Koffi,
    /// ref-napi + ffi-napi
    RefNapi,
}

impl FfiBackend {
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "koffi" => Some(Self::Koffi),
            "ref-napi" => Some(Self::RefNapi),
            _ => None,
        }
    }

    pub fn _as_str(&self) -> &'static str {
        match self {
            Self::Koffi => "koffi",
            Self::RefNapi => "ref-napi",
        }
    }
}

impl Default for FfiBackend {
    fn default() -> Self {
        Self::Koffi
    }
}
