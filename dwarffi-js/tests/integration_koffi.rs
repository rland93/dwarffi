use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use log::{debug, error, info, warn};

#[test]
#[cfg(target_os = "macos")]
fn test_koffi_bindings_end_to_end() {
    let _ = env_logger::builder()
        .is_test(true)
        .filter_level(log::LevelFilter::Debug)
        .try_init();

    info!("Starting Koffi bindings integration test");

    // check node is available
    if Command::new("node").arg("--version").output().is_err() {
        warn!("Node.js not found in PATH - skipping integration test");
        return;
    }

    // workspace root (dwarffi/)
    let workspace_root = get_workspace_root();
    debug!("Workspace root: {:?}", workspace_root);

    // build C test library
    info!("Building C test library");
    build_test_library(&workspace_root);

    // generate bindings using dwarffi-js
    info!("Generating Koffi bindings");
    let bindings_code = generate_bindings(&workspace_root);
    debug!("Generated {} bytes of bindings", bindings_code.len());

    // create temp dir for the test
    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
    let temp_path = temp_dir.path().to_path_buf();
    debug!("Using temp directory: {:?}", temp_path);

    // write generated bindings
    let bindings_path = temp_path.join("bindings.js");
    fs::write(&bindings_path, &bindings_code).expect("Failed to write bindings.js");
    debug!("Wrote bindings to: {:?}", bindings_path);

    // update LIBRARY_PATH in bindings
    update_library_path(&bindings_path, &workspace_root);

    // copy the test runner script
    let test_script_source = workspace_root
        .join("dwarffi-js")
        .join("tests")
        .join("test-koffi-bindings.mjs");
    let test_script_dest = temp_path.join("test.mjs");
    fs::copy(&test_script_source, &test_script_dest).expect("Failed to copy test script");
    debug!("Copied test script to: {:?}", test_script_dest);

    // install koffi in the temp directory
    info!("Installing koffi dependency");
    install_koffi(&temp_path);

    // run the Node.js tests with TAP output
    info!("Running Node.js test suite");
    let output = Command::new("node")
        .args(&["--test", "--test-reporter=tap", "test.mjs"])
        .current_dir(&temp_path)
        .output()
        .expect("Failed to execute Node.js tests");

    // parse and log TAP output
    let tap_output = String::from_utf8_lossy(&output.stdout);
    debug!("Raw TAP output:\n{}", tap_output);

    // Simple TAP parser - handles both version 13 and 14
    let mut passed = 0;
    let mut failed = 0;
    let mut failed_tests = Vec::new();
    let mut plan_count: Option<usize> = None;

    for line in tap_output.lines() {
        let trimmed = line.trim();

        // TAP version line (13 or 14)
        if trimmed.starts_with("TAP version") {
            debug!("{}", trimmed);
            continue;
        }

        // test plan: "1..N"
        if let Some(plan_str) = trimmed.strip_prefix("1..") {
            if let Ok(count) = plan_str.trim().parse::<usize>() {
                plan_count = Some(count);
                info!("TAP test plan: {} tests", count);
            }
            continue;
        }

        // test result: "ok N - description" or "not ok N - description"
        // skip indented subtests (they're counted in the parent)
        if !line.starts_with("    ") {
            if trimmed.starts_with("ok ") {
                // parse test number and description
                let rest = trimmed.strip_prefix("ok ").unwrap();
                let (num, desc) = parse_test_line(rest);
                info!("  ✓ Test {}: {}", num, desc);
                passed += 1;
            } else if trimmed.starts_with("not ok ") {
                let rest = trimmed.strip_prefix("not ok ").unwrap();
                let (num, desc) = parse_test_line(rest);
                error!("  ✗ Test {}: {}", num, desc);
                failed += 1;
                failed_tests.push(desc.to_string());
            }
        }

        // comments and diagnostics
        if trimmed.starts_with("#") {
            debug!("{}", trimmed);
        }

        // bail out
        if trimmed.starts_with("Bail out!") {
            error!("TAP bail out: {}", trimmed);
            panic!("Test suite bailed out");
        }
    }

    // verify plan if present
    if let Some(expected) = plan_count {
        let actual = passed + failed;
        if actual != expected {
            warn!(
                "Test count mismatch: expected {}, got {} (passed: {}, failed: {})",
                expected, actual, passed, failed
            );
        }
    }

    // log Node.js stderr if present
    if !output.stderr.is_empty() {
        error!("Node.js stderr:");
        for line in String::from_utf8_lossy(&output.stderr).lines() {
            error!("  {}", line);
        }
    }

    // summary
    info!("Test results: {} passed, {} failed", passed, failed);

    if failed > 0 {
        error!("Failed tests:");
        for test_name in &failed_tests {
            error!("  - {}", test_name);
        }
        error!("Temp directory preserved at: {:?}", temp_path);
        error!("Inspect generated bindings: {}", bindings_path.display());

        // prevent cleanup on failure
        std::mem::forget(temp_dir);

        panic!("{} test(s) failed", failed);
    }

    info!("✓ All {} integration tests passed!", passed);
}

/// parse a TAP test line to extract test number and description
///
/// input: "1 - test description" or "1 - test description # SKIP reason"
///
/// output: (test_number, description)
fn parse_test_line(line: &str) -> (usize, &str) {
    let line = line.trim();

    // split on " - " to separate number from description
    if let Some(dash_pos) = line.find(" - ") {
        let num_str = line[..dash_pos].trim();
        let desc = line[dash_pos + 3..].trim();

        // remove directives (# SKIP, # TODO, etc.)
        let desc_clean = if let Some(hash_pos) = desc.find(" #") {
            desc[..hash_pos].trim()
        } else {
            desc
        };

        let num = num_str.parse().unwrap_or(0);
        (num, desc_clean)
    } else {
        // no description, just number
        let num = line
            .split_whitespace()
            .next()
            .and_then(|s| s.parse().ok())
            .unwrap_or(0);
        (num, "")
    }
}

/// get the workspace root directory (dwarffi/)
fn get_workspace_root() -> PathBuf {
    let manifest_dir = env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR not set");
    PathBuf::from(manifest_dir)
        .parent()
        .expect("Failed to get parent directory")
        .to_path_buf()
}

/// build the C test library using make
fn build_test_library(workspace_root: &Path) {
    let test_c_dir = workspace_root.join("test_c");
    debug!("Building C library in: {:?}", test_c_dir);

    // check if make is available
    if Command::new("make").arg("--version").output().is_err() {
        panic!("make not found in PATH - cannot build test library");
    }

    // run make in the test_c directory
    let status = Command::new("make")
        .current_dir(&test_c_dir)
        .status()
        .expect("Failed to run make");

    if !status.success() {
        panic!("Failed to build test library");
    }

    // verify the library was built
    let lib_path = test_c_dir.join("libtestlib.dylib");
    if !lib_path.exists() {
        panic!("Test library not found after build: {:?}", lib_path);
    }
    debug!("Built test library: {:?}", lib_path);
}

/// generate JavaScript bindings using dwarffi-js CLI
fn generate_bindings(workspace_root: &Path) -> String {
    let testlib_path = workspace_root.join("test_c").join("testlib.o");

    if !testlib_path.exists() {
        panic!("testlib.o not found: {:?}", testlib_path);
    }

    debug!("Generating bindings from: {:?}", testlib_path);

    let output = Command::new("cargo")
        .args(&[
            "run",
            "--package",
            "dwarffi-js",
            "--",
            testlib_path.to_str().unwrap(),
            "--js",
            "--functions",
            "--all",
            "--library-path",
            "./libtestlib.dylib", // Will be updated to absolute path
        ])
        .current_dir(workspace_root)
        .output()
        .expect("Failed to run dwarffi-js");

    if !output.status.success() {
        error!(
            "dwarffi-js stderr: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        panic!("Failed to generate bindings");
    }

    String::from_utf8(output.stdout).expect("Invalid UTF-8 in bindings")
}

/// install koffi package using npm
fn install_koffi(dir: &Path) {
    // Check if npm is available
    if Command::new("npm").arg("--version").output().is_err() {
        panic!("npm not found in PATH - cannot install koffi");
    }

    debug!("Installing koffi in: {:?}", dir);

    // run npm install koffi
    let status = Command::new("npm")
        .args(&["install", "koffi", "--silent"])
        .current_dir(dir)
        .status()
        .expect("Failed to run npm install");

    if !status.success() {
        panic!("Failed to install koffi");
    }

    debug!("Koffi installed successfully");
}

/// update the LIBRARY_PATH constant in the generated bindings to use absolute path
fn update_library_path(bindings_path: &Path, workspace_root: &Path) {
    let content = fs::read_to_string(bindings_path).expect("Failed to read bindings.js");

    let lib_path = workspace_root
        .join("test_c")
        .join("libtestlib.dylib")
        .canonicalize()
        .expect("Failed to get absolute path for library");

    debug!("Setting library path to: {:?}", lib_path);

    // replace the LIBRARY_PATH line
    let updated = content.replace(
        "const LIBRARY_PATH = './libtestlib.dylib'",
        &format!("const LIBRARY_PATH = '{}'", lib_path.display()),
    );

    fs::write(bindings_path, updated).expect("Failed to update bindings.js");
}
