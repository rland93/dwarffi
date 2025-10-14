use ffitool::DwarfAnalyzer;
use std::path::Path;

#[test]
fn test_extract_types_from_testlib() {
    // Load the test library (use dSYM bundle for DWARF info)
    let analyzer = DwarfAnalyzer::from_file(Path::new(
        "test_c/libtestlib.dylib.dSYM/Contents/Resources/DWARF/libtestlib.dylib"
    )).expect("Failed to load test library");

    // Extract type registry (exported functions only)
    let registry = analyzer
        .extract_type_registry(true)
        .expect("Failed to extract type registry");

    // Should have some types
    assert!(
        !registry.is_empty(),
        "Type registry should not be empty. Size: {}",
        registry.len()
    );

    // Should have primitive types like int
    let int_types = registry.get_by_name("int");
    assert!(
        !int_types.is_empty(),
        "Should have found 'int' primitive type"
    );

    // Find the base int type (not a pointer)
    let int_type = int_types
        .iter()
        .find(|t| t.pointer_depth == 0 && !t.is_const && !t.is_volatile)
        .expect("Should have found base 'int' type");

    assert_eq!(int_type.pointer_depth, 0, "int should not be a pointer");
    assert!(!int_type.is_const, "int should not be const");

    println!("✓ Type registry extracted successfully");
    println!("  Total types: {}", registry.len());
}

#[test]
fn test_compare_with_string_extraction() {
    let analyzer = DwarfAnalyzer::from_file(Path::new(
        "test_c/libtestlib.dylib.dSYM/Contents/Resources/DWARF/libtestlib.dylib"
    )).expect("Failed to load test library");

    // Extract both string signatures and type registry
    let signatures = analyzer
        .extract_signatures(true)
        .expect("Failed to extract signatures");

    let registry = analyzer
        .extract_type_registry(true)
        .expect("Failed to extract type registry");

    // Both should work
    assert!(!signatures.is_empty(), "Should have function signatures");
    assert!(!registry.is_empty(), "Should have type registry");

    println!("✓ Both extraction methods work");
    println!("  Functions: {}", signatures.len());
    println!("  Types: {}", registry.len());
}
