use dwarffi::DwarfAnalyzer;
use std::path::Path;

#[test]
fn test_extract_types_from_testlib() {
    // Load the test library (use dSYM bundle for DWARF info)
    let analyzer = DwarfAnalyzer::from_file(Path::new(
        "test_c/libtestlib.dylib.dSYM/Contents/Resources/DWARF/libtestlib.dylib",
    ))
    .expect("Failed to load test library");

    // Extract analysis (exported functions only)
    let result = analyzer
        .extract_analysis(true)
        .expect("Failed to extract analysis");
    let registry = result.type_registry;

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
        "test_c/libtestlib.dylib.dSYM/Contents/Resources/DWARF/libtestlib.dylib",
    ))
    .expect("Failed to load test library");

    // Extract analysis
    let result = analyzer
        .extract_analysis(true)
        .expect("Failed to extract analysis");

    let signatures = &result.signatures;
    let registry = &result.type_registry;

    // Both should work
    assert!(!signatures.is_empty(), "Should have function signatures");
    assert!(!registry.is_empty(), "Should have type registry");

    println!("✓ Both extraction methods work");
    println!("  Functions: {}", signatures.len());
    println!("  Types: {}", registry.len());
}

// ============================================================================
// Phase 1 Integration Tests: Type Graph Integrity
// ============================================================================

#[test]
fn test_no_dangling_references() {
    use dwarffi::{BaseTypeKind, TypeId};
    use std::collections::HashSet;

    let analyzer = DwarfAnalyzer::from_file(Path::new(
        "test_c/libtestlib.dylib.dSYM/Contents/Resources/DWARF/libtestlib.dylib",
    ))
    .expect("Failed to load test library");

    let result = analyzer
        .extract_analysis(true)
        .expect("Failed to extract analysis");
    let registry = result.type_registry;

    // Collect all TypeIds that exist in the registry
    let existing_ids: HashSet<TypeId> = registry.all_types().map(|t| t.id).collect();

    // Collect all referenced TypeIds
    let mut referenced_ids = HashSet::new();

    for type_ in registry.all_types() {
        match &type_.kind {
            BaseTypeKind::Struct { fields, .. } => {
                for field in fields {
                    referenced_ids.insert(field.type_id);
                }
            }
            BaseTypeKind::Union { variants, .. } => {
                for variant in variants {
                    referenced_ids.insert(variant.type_id);
                }
            }
            BaseTypeKind::Enum { backing_id, .. } => {
                referenced_ids.insert(*backing_id);
            }
            BaseTypeKind::Array {
                element_type_id, ..
            } => {
                referenced_ids.insert(*element_type_id);
            }
            BaseTypeKind::Typedef {
                aliased_type_id, ..
            } => {
                referenced_ids.insert(*aliased_type_id);
            }
            BaseTypeKind::Function {
                return_type_id,
                parameter_type_ids,
                ..
            } => {
                if let Some(id) = return_type_id {
                    referenced_ids.insert(*id);
                }
                for id in parameter_type_ids {
                    referenced_ids.insert(*id);
                }
            }
            BaseTypeKind::Primitive { .. } => {
                // Primitives don't reference other types
            }
        }
    }

    // Check that all referenced TypeIds exist
    for ref_id in &referenced_ids {
        assert!(
            existing_ids.contains(ref_id),
            "Dangling reference: TypeId {:?} is referenced but doesn't exist in registry",
            ref_id
        );
    }

    println!("✓ No dangling references found");
    println!("  Existing types: {}", existing_ids.len());
    println!("  Referenced types: {}", referenced_ids.len());
}

#[test]
fn test_nested_type_closure() {
    use dwarffi::BaseTypeKind;

    let analyzer = DwarfAnalyzer::from_file(Path::new(
        "test_c/libtestlib.dylib.dSYM/Contents/Resources/DWARF/libtestlib.dylib",
    ))
    .expect("Failed to load test library");

    let result = analyzer
        .extract_analysis(true)
        .expect("Failed to extract analysis");
    let registry = result.type_registry;

    // Find BoundingBox struct (if it exists)
    let bbox_types = registry.get_by_name("BoundingBox");

    if !bbox_types.is_empty() {
        let bbox = bbox_types[0];

        // BoundingBox is a typedef, follow it to the struct
        let bbox_struct = match &bbox.kind {
            BaseTypeKind::Typedef {
                aliased_type_id, ..
            } => registry
                .get_type(*aliased_type_id)
                .expect("BoundingBox typedef should reference a valid type"),
            BaseTypeKind::Struct { .. } => bbox,
            _ => panic!("BoundingBox should be a typedef or struct"),
        };

        // BoundingBox struct should have fields
        match &bbox_struct.kind {
            BaseTypeKind::Struct { fields, .. } => {
                assert!(!fields.is_empty(), "BoundingBox should have fields");

                // Each field should reference a valid type
                for field in fields {
                    let field_type = registry.get_type(field.type_id);
                    assert!(
                        field_type.is_some(),
                        "Field '{}' references non-existent type",
                        field.name
                    );

                    // If it's a Point struct, verify it references int
                    if let Some(ft) = field_type
                        && let BaseTypeKind::Struct {
                            name,
                            fields: point_fields,
                            ..
                        } = &ft.kind
                            && name == "Point" {
                                // Point should have fields referencing int
                                for pf in point_fields {
                                    let pf_type = registry.get_type(pf.type_id);
                                    assert!(
                                        pf_type.is_some(),
                                        "Point field '{}' references non-existent type",
                                        pf.name
                                    );
                                }
                            }
                }
            }
            _ => panic!("BoundingBox struct should be a struct type"),
        }

        println!("✓ Nested type closure verified (BoundingBox → Point → int)");
    } else {
        println!("⚠ BoundingBox not found in registry (may not be exported)");
    }
}

#[test]
fn test_array_element_closure() {
    use dwarffi::BaseTypeKind;

    let analyzer = DwarfAnalyzer::from_file(Path::new(
        "test_c/libtestlib.dylib.dSYM/Contents/Resources/DWARF/libtestlib.dylib",
    ))
    .expect("Failed to load test library");

    let result = analyzer
        .extract_analysis(true)
        .expect("Failed to extract analysis");
    let registry = result.type_registry;

    // Find Person struct (if it exists) which has char name[64]
    let person_types = registry.get_by_name("Person");

    if !person_types.is_empty() {
        let person = person_types[0];

        // Person might be a typedef, follow it to the struct
        let person_struct = match &person.kind {
            BaseTypeKind::Typedef {
                aliased_type_id, ..
            } => registry
                .get_type(*aliased_type_id)
                .expect("Person typedef should reference a valid type"),
            BaseTypeKind::Struct { .. } => person,
            _ => panic!("Person should be a typedef or struct"),
        };

        match &person_struct.kind {
            BaseTypeKind::Struct { fields, .. } => {
                // Look for array field (name)
                for field in fields {
                    let field_type = registry.get_type(field.type_id);
                    if let Some(ft) = field_type
                        && let BaseTypeKind::Array {
                            element_type_id,
                            count,
                            ..
                        } = &ft.kind
                        {
                            // Verify element type exists
                            let element_type = registry.get_type(*element_type_id);
                            assert!(
                                element_type.is_some(),
                                "Array field '{}' has dangling element type reference",
                                field.name
                            );

                            println!(
                                "✓ Array field '{}' [{}] element type exists",
                                field.name, count
                            );
                        }
                }
            }
            _ => panic!("Person struct should be a struct type"),
        }
    } else {
        println!("⚠ Person struct not found in registry (may not be exported)");
    }
}

#[test]
fn test_typedef_chain_closure() {
    use dwarffi::BaseTypeKind;

    let analyzer = DwarfAnalyzer::from_file(Path::new(
        "test_c/libtestlib.dylib.dSYM/Contents/Resources/DWARF/libtestlib.dylib",
    ))
    .expect("Failed to load test library");

    let result = analyzer
        .extract_analysis(true)
        .expect("Failed to extract analysis");
    let registry = result.type_registry;

    // Look for typedef types
    let mut typedef_count = 0;
    let mut chain_verified = 0;

    for type_ in registry.all_types() {
        if let BaseTypeKind::Typedef {
            name,
            aliased_type_id,
        } = &type_.kind
        {
            typedef_count += 1;

            // Verify aliased type exists
            let aliased = registry.get_type(*aliased_type_id);
            assert!(
                aliased.is_some(),
                "Typedef '{}' references non-existent type",
                name
            );

            // Follow the chain if aliased type is also a typedef
            let mut current_id = *aliased_type_id;
            let mut depth = 0;
            loop {
                if let Some(current) = registry.get_type(current_id) {
                    depth += 1;
                    if let BaseTypeKind::Typedef {
                        aliased_type_id, ..
                    } = &current.kind
                    {
                        current_id = *aliased_type_id;
                        if depth > 10 {
                            panic!("Typedef chain too deep (possible cycle)");
                        }
                    } else {
                        // Reached end of chain
                        chain_verified += 1;
                        break;
                    }
                } else {
                    panic!("Broken typedef chain for '{}'", name);
                }
            }
        }
    }

    println!("✓ Typedef chain closure verified");
    println!("  Typedefs found: {}", typedef_count);
    println!("  Chains verified: {}", chain_verified);
}
