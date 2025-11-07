//! Shared test utilities for platform-portable test library path handling

use std::path::{Path, PathBuf};

/// return the path to the test C library with DWARF debug info.
///
/// macOS -> DWARF file inside the dSYM bundle.
/// Linux -> .so file with embedded debug info.
///
/// # Panics
/// panics if the path doesn't exist. Make sure to build the test library first:
/// ```bash
/// cd test_c && make
/// # On macOS only:
/// dsymutil test_c/libtestlib.dylib
/// ```
#[cfg_attr(not(target_os = "macos"), allow(dead_code))]
pub fn get_test_lib_path() -> PathBuf {
    let path = get_test_lib_path_unchecked();

    if !path.exists() {
        panic!(
            "Test library not found at: {}\n\
             Please build it first:\n\
             cd test_c && make clean && make\n\
             {}",
            path.display(),
            if cfg!(target_os = "macos") {
                "dsymutil test_c/libtestlib.dylib"
            } else {
                ""
            }
        );
    }

    path
}

/// return the test library path without checking if it exists.
/// use `get_test_lib_path()` for the version with validation.
pub fn get_test_lib_path_unchecked() -> PathBuf {
    #[cfg(target_os = "macos")]
    {
        get_test_lib_dir()
            .join("libtestlib.dylib.dSYM")
            .join("Contents/Resources/DWARF/libtestlib.dylib")
    }

    #[cfg(target_os = "linux")]
    {
        get_test_lib_dir().join("libtestlib.so")
    }

    #[cfg(not(any(target_os = "macos", target_os = "linux")))]
    {
        compile_error!("Unsupported platform for test library");
    }
}

/// return the directory containing the test C library.
pub fn get_test_lib_dir() -> PathBuf {
    // up one level from dwarffi -> workspace root -> test_c
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("Failed to get parent directory of CARGO_MANIFEST_DIR")
        .join("test_c")
}

/// return the path to the dynamic library for FFI loading (not debug symbols).
///
/// macOS -> .dylib file.
/// Linux -> .so file.
pub fn get_test_dylib_path() -> PathBuf {
    #[cfg(target_os = "macos")]
    {
        get_test_lib_dir().join("libtestlib.dylib")
    }

    #[cfg(target_os = "linux")]
    {
        get_test_lib_dir().join("libtestlib.so")
    }

    #[cfg(not(any(target_os = "macos", target_os = "linux")))]
    {
        compile_error!("Unsupported platform for test library");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lib_dir_exists() {
        let dir = get_test_lib_dir();
        assert!(
            dir.exists(),
            "test_c directory should exist at: {}",
            dir.display()
        );
    }

    #[test]
    fn test_lib_path_format() {
        let path = get_test_lib_path_unchecked();

        #[cfg(target_os = "macos")]
        {
            assert!(path.to_string_lossy().contains("libtestlib.dylib.dSYM"));
            assert!(path.to_string_lossy().ends_with("libtestlib.dylib"));
        }

        #[cfg(target_os = "linux")]
        {
            assert!(path.to_string_lossy().ends_with("libtestlib.so"));
        }
    }

    #[test]
    fn test_dylib_path_format() {
        let path = get_test_dylib_path();

        #[cfg(target_os = "macos")]
        {
            assert!(path.to_string_lossy().ends_with("libtestlib.dylib"));
            // Should NOT contain .dSYM
            assert!(!path.to_string_lossy().contains(".dSYM"));
        }

        #[cfg(target_os = "linux")]
        {
            assert!(path.to_string_lossy().ends_with("libtestlib.so"));
        }
    }
}
