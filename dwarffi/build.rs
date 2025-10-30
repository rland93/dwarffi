/// requirements for building+running integration tests:
///
/// * C compiler (e.g. gcc, clang)
/// * make
/// * for macOS: dsymutil (optional, for dSYM tests)
///
/// for linux
///
/// `sudo apt install build-essential`
///
/// for macOS
///
/// `xcode-select --install`
///
use std::process::Command;

fn main() {
    // rerun build if the test C library source files change.
    println!("cargo:rerun-if-changed=../test_c/testlib.c");
    println!("cargo:rerun-if-changed=../test_c/testlib.h");
    println!("cargo:rerun-if-changed=../test_c/makefile");

    // build the test c library
    let status = Command::new("make")
        .current_dir("../test_c")
        .status()
        .expect("Failed to execute make - ensure make is installed");

    if !status.success() {
        panic!("Failed to compile test C library");
    }

    // macos -- dsymutil necessary to create dSYM bundle.
    #[cfg(target_os = "macos")]
    {
        let dylib_path = "../test_c/libtestlib.dylib";
        let result = Command::new("dsymutil").arg(dylib_path).status();

        match result {
            Ok(status) if status.success() => {
                println!("cargo:warning=Created dSYM bundle for testing");
            }
            Ok(_) => {
                println!("cargo:warning=dsymutil failed, dSYM tests may fail");
            }
            Err(_) => {
                println!("cargo:warning=dsymutil not found, dSYM tests will be skipped");
            }
        }
    }
}
