/// verify that all primitive types from test_c library are correctly handled by
/// the Koffi code generator.
///
/// 1. parses the test_c library DWARF info
/// 2. extracts all function signatures
/// 3. verifies that all primitive types used in those signatures can be
///    successfully mapped to Koffi types
mod common;

use dwarffi::{BaseTypeKind, DwarfAnalyzer};
use std::collections::HashSet;
use std::process::Command;

#[test]
#[cfg(target_os = "macos")]
fn test_all_primitive_types_from_test_library() {
    let _ = env_logger::builder()
        .is_test(true)
        .filter_level(log::LevelFilter::Debug)
        .try_init();

    let lib_path = common::get_test_lib_path();

    if !lib_path.exists() {
        let workspace_root = common::get_workspace_root();
        let test_c_dir = workspace_root.join("test_c");
        std::process::Command::new("make")
            .current_dir(&test_c_dir)
            .status()
            .expect("Failed to build test library");
    }

    assert!(
        lib_path.exists(),
        "Test library not found at {:?}",
        lib_path
    );

    let analyzer = DwarfAnalyzer::from_file(&lib_path).expect("Failed to create analyzer");

    let analysis = analyzer
        .extract_analysis(false) // include all functions, not just exported
        .expect("Failed to extract analysis");

    let type_registry = analysis.type_registry;
    let signatures = analysis.signatures;

    println!("Extracted {} function signatures", signatures.len());
    println!("Type registry has {} types", type_registry.len());

    let mut primitive_types = HashSet::new();

    for sig in &signatures {
        // Check return type
        if let Some(type_info) = type_registry.get_type(sig.return_type_id)
            && let BaseTypeKind::Primitive { name, .. } = &type_info.kind
        {
            primitive_types.insert(name.clone());
        }

        for param in &sig.parameters {
            if let Some(type_info) = type_registry.get_type(param.type_id)
                && let BaseTypeKind::Primitive { name, .. } = &type_info.kind
            {
                primitive_types.insert(name.clone());
            }
        }
    }

    println!("Found {} unique primitive types:", primitive_types.len());
    let mut sorted_types: Vec<_> = primitive_types.iter().collect();
    sorted_types.sort();
    for type_name in &sorted_types {
        println!("  - {}", type_name);
    }

    let workspace_root = common::get_workspace_root();
    let output = Command::new("cargo")
        .args([
            "run",
            "--package",
            "dwarffi-js",
            "--",
            lib_path.to_str().unwrap(),
            "--js",
            "--functions",
            "--library-path",
            "./libtestlib.dylib",
        ])
        .current_dir(&workspace_root)
        .output()
        .expect("Failed to run dwarffi-js");

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        panic!("Failed to generate Koffi bindings:\n{}", stderr);
    }

    let bindings = String::from_utf8(output.stdout).expect("Invalid UTF-8 in bindings");

    println!(
        "✓ Successfully generated {} bytes of bindings",
        bindings.len()
    );
    assert!(
        !bindings.is_empty(),
        "Generated bindings should not be empty"
    );

    // bindings contain primitive types
    assert!(
        bindings.contains("void"),
        "Bindings should contain void type"
    );
    assert!(bindings.contains("int"), "Bindings should contain int type");
    assert!(
        bindings.contains("float"),
        "Bindings should contain float type"
    );
    assert!(
        bindings.contains("double"),
        "Bindings should contain double type"
    );
}

#[test]
#[cfg(target_os = "macos")]
fn test_comprehensive_primitive_coverage() {
    let lib_path = common::get_test_lib_path();

    if !lib_path.exists() {
        let workspace_root = common::get_workspace_root();
        let test_c_dir = workspace_root.join("test_c");
        std::process::Command::new("make")
            .current_dir(&test_c_dir)
            .status()
            .expect("Failed to build test library");
    }

    let analyzer = DwarfAnalyzer::from_file(&lib_path).expect("Failed to create analyzer");
    let analysis = analyzer
        .extract_analysis(true) // only exported functions
        .expect("Failed to extract analysis");

    let signatures = analysis.signatures;

    let expected_functions = vec![
        "get_char",
        "get_signed_char",
        "get_unsigned_char",
        "get_short",
        "get_unsigned_short",
        "get_int",
        "get_unsigned_int",
        "get_long",
        "get_unsigned_long",
        "get_long_long",
        "get_unsigned_long_long",
        "get_float",
        "get_double",
        "get_long_double",
        "get_bool",
    ];

    let found_functions: HashSet<_> = signatures.iter().map(|s| s.name.as_str()).collect();

    for func_name in &expected_functions {
        assert!(
            found_functions.contains(func_name),
            "Expected function '{}' not found in signatures",
            func_name
        );
    }

    println!(
        "✓ Found all {} expected primitive type functions",
        expected_functions.len()
    );

    let workspace_root = common::get_workspace_root();
    let output = Command::new("cargo")
        .args([
            "run",
            "--package",
            "dwarffi-js",
            "--",
            lib_path.to_str().unwrap(),
            "--js",
            "--functions",
            "--library-path",
            "./libtestlib.dylib",
        ])
        .current_dir(&workspace_root)
        .output()
        .expect("Failed to run dwarffi-js");

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        panic!("Failed to generate bindings:\n{}", stderr);
    }

    let bindings = String::from_utf8(output.stdout).expect("Invalid UTF-8 in bindings");

    for func_name in &expected_functions {
        assert!(
            bindings.contains(func_name),
            "Generated bindings should contain function '{}'",
            func_name
        );
    }

    println!("✓ All primitive type functions present in generated bindings");
}
