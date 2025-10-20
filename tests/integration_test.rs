use ffitool::DwarfAnalyzer;
use std::path::PathBuf;

/// get test library path per platform
fn get_test_lib_path() -> PathBuf {
    #[cfg(target_os = "macos")]
    {
        PathBuf::from("test_c/libtestlib.dylib")
    }
    #[cfg(target_os = "linux")]
    {
        PathBuf::from("test_c/libtestlib.so")
    }
}

/// in macOS, the annotated library w/ DWARF info is stored in the dSYM bundle
/// otherwise the binary is stripped of debug info.
#[cfg(target_os = "macos")]
fn get_dsym_path() -> PathBuf {
    PathBuf::from("test_c/libtestlib.dylib.dSYM/Contents/Resources/DWARF/libtestlib.dylib")
}

/// expected functions from test C lib.
///
/// TODO FIXME!! requires manual syncing. maybe an annotation in the comment of
/// the c source,  can be used to auto-generate this list when the test runs?
const EXPECTED_SIGNATURES: &[&str] = &[
    "Point add_points(Point p1, Point p2)",
    "int add_two_ints(int a, int b)",
    "int* allocate_array(size_t count)",
    "void allocate_matrix(int** matrix, int rows, int cols)",
    "Color blend_colors(Color c1, Color c2)",
    "float calculate_distance(Point p1, Point p2)",
    "void cleanup_state(InternalState* state)",
    "void complex_function(const char* name, Point* points, size_t point_count, Rectangle bounds, Status* out_status)",
    "double compute_double(double x, double y, double z)",
    "BoundingBox create_bounding_box(Point tl, Point br)",
    "DataUnion create_data_union(int value)",
    "Person* create_person(const char* name, int age)",
    "Point create_point(int x, int y)",
    "Rectangle create_rectangle(float w, float h)",
    "void destroy_person(Person* p)",
    "float get_float_from_union(DataUnion data)",
    "size_t get_size(void)",
    "Status get_status(void)",
    "const char* get_string(void)",
    "InternalState* init_state(void)",
    "int internal_compute(int a, int b)",
    "void internal_helper(void)",
    "void internal_process_data(const char* data, size_t len)",
    "int is_point_inside(BoundingBox box, Point p)",
    "void modify_value(int* ptr)",
    "void move_point(Point* p, int dx, int dy)",
    "float multiply_floats(float a, float b)",
    "void print_string(const char* str)",
    "void process_2d_array(int[5]* arr)",
    "void process_buffer(char* buffer, size_t length)",
    "uint8_t process_byte(uint8_t value)",
    "void process_fixed_array(int* arr)",
    "int64_t process_long(int64_t value)",
    "Status process_person_batch(Person** people, size_t count, Callback on_complete)",
    "int process_state(InternalState* state, int value)",
    "void register_callback(Callback cb, void* userdata)",
    "int return_int(void)",
    "void set_status(Status s)",
    "void simple_void_function(void)",
    "void sort_array(int* arr, size_t count, Comparator cmp)",
    "int sum_array(const int* arr, size_t length)",
    "int sum_varargs(int count, ...)",
    "void update_person_status(Person* p, Status new_status)",
];

#[test]
/// load files
fn test_load_object_file() {
    let path = PathBuf::from("test_c/testlib.o");
    DwarfAnalyzer::from_file(&path).expect("fail to load object file");
}

#[test]
/// load files
fn test_load_shared_library() {
    let path = get_test_lib_path();
    DwarfAnalyzer::from_file(&path).expect("fail to load shared library");
}

#[test]
/// load files, dSYM on macOS
#[cfg(target_os = "macos")]
fn test_load_dsym() {
    let path = get_dsym_path();
    if path.exists() {
        DwarfAnalyzer::from_file(&path).expect("fail to load dSYM file");
    }
}

#[test]
/// test error on nonexistent file
fn test_error_on_nonexistent_file() {
    let path = PathBuf::from("nonexistent/library.dylib");
    let result = DwarfAnalyzer::from_file(&path);
    assert!(result.is_err(), "Should fail on nonexistent file");
}

#[test]
/// accurately count functions in the test lib
fn test_function_count_all() {
    let path = PathBuf::from("test_c/testlib.o");
    let analyzer = DwarfAnalyzer::from_file(&path).expect("fail to load test library");
    let result = analyzer
        .extract_analysis(false)
        .expect("fail to extract analysis");

    assert_eq!(
        result.signatures.len(),
        43,
        "expect 43 functions, found {}",
        result.signatures.len()
    );
}

#[test]
/// accurately count exported functions in the test lib
fn test_function_count_exported() {
    let path = PathBuf::from("test_c/testlib.o");
    let analyzer = DwarfAnalyzer::from_file(&path).expect("fail to load test library");
    let result = analyzer
        .extract_analysis(true)
        .expect("fail to extract analysis");

    // All 43 functions in testlib are exported
    assert_eq!(
        result.signatures.len(),
        43,
        "Expected 43 exported functions, found {}",
        result.signatures.len()
    );
}

#[test]
/// go thru list of expected signatures (found above) and verify that
/// the strings match. a little crude because this also tests to_string
fn test_all_expected_signatures_present() {
    let path = PathBuf::from("test_c/testlib.o");
    let analyzer = DwarfAnalyzer::from_file(&path).expect("fail to load test library");
    let result = analyzer
        .extract_analysis(false)
        .expect("fail to extract analysis");

    let sig_strings: Vec<String> = result.signatures.iter().map(|s| s.to_string(&result.type_registry)).collect();

    for expected in EXPECTED_SIGNATURES {
        assert!(
            sig_strings.iter().any(|s| s == expected),
            "missing expected signature: {}",
            expected
        );
    }
}

#[test]
/// test simple void function signature
fn test_simple_void_function_signature() {
    let path = PathBuf::from("test_c/testlib.o");
    let analyzer = DwarfAnalyzer::from_file(&path).expect("fail to load test library");
    let result = analyzer
        .extract_analysis(false)
        .expect("fail to extract analysis");

    let sig = result.signatures
        .iter()
        .find(|s| s.name == "simple_void_function")
        .expect("simple_void_function not found");

    let return_type_str = result.type_registry.get_type(sig.return_type_id)
        .map(|t| t.to_c_string(&result.type_registry))
        .unwrap_or_else(|| "void".to_string());
    assert_eq!(return_type_str, "void");
    assert_eq!(sig.parameters.len(), 0);
    assert_eq!(sig.is_variadic, false);
    assert_eq!(sig.to_string(&result.type_registry), "void simple_void_function(void)");
}

#[test]
/// test primitive parameters signature
fn test_primitive_parameters_signature() {
    let path = PathBuf::from("test_c/testlib.o");
    let analyzer = DwarfAnalyzer::from_file(&path).expect("fail to load test library");
    let result = analyzer
        .extract_analysis(false)
        .expect("fail to extract analysis");

    let sig = result.signatures
        .iter()
        .find(|s| s.name == "add_two_ints")
        .expect("add_two_ints not found");

    let return_type_str = result.type_registry.get_type(sig.return_type_id)
        .map(|t| t.to_c_string(&result.type_registry))
        .unwrap_or_else(|| "void".to_string());
    assert_eq!(return_type_str, "int");
    assert_eq!(sig.parameters.len(), 2);
    assert_eq!(sig.parameters[0].name, "a");
    let param0_type = result.type_registry.get_type(sig.parameters[0].type_id)
        .map(|t| t.to_c_string(&result.type_registry))
        .unwrap_or_else(|| "void".to_string());
    assert_eq!(param0_type, "int");
    assert_eq!(sig.parameters[1].name, "b");
    let param1_type = result.type_registry.get_type(sig.parameters[1].type_id)
        .map(|t| t.to_c_string(&result.type_registry))
        .unwrap_or_else(|| "void".to_string());
    assert_eq!(param1_type, "int");
    assert_eq!(sig.to_string(&result.type_registry), "int add_two_ints(int a, int b)");
}

#[test]
/// test pointer types signature
fn test_pointer_types_signature() {
    let path = PathBuf::from("test_c/testlib.o");
    let analyzer = DwarfAnalyzer::from_file(&path).expect("Failed to load test library");
    let result = analyzer
        .extract_analysis(false)
        .expect("fail to extract analysis");

    let sig = result.signatures
        .iter()
        .find(|s| s.name == "get_string")
        .expect("get_string not found");

    let return_type_str = result.type_registry.get_type(sig.return_type_id)
        .map(|t| t.to_c_string(&result.type_registry))
        .unwrap_or_else(|| "void".to_string());
    assert_eq!(return_type_str, "const char*");
    assert_eq!(sig.parameters.len(), 0);
}

#[test]
/// test struct types signature
fn test_struct_types_signature() {
    let path = PathBuf::from("test_c/testlib.o");
    let analyzer = DwarfAnalyzer::from_file(&path).expect("fail to load test library");
    let result = analyzer
        .extract_analysis(false)
        .expect("fail to extract analysis");

    let sig = result.signatures
        .iter()
        .find(|s| s.name == "create_point")
        .expect("create_point not found");

    let return_type_str = result.type_registry.get_type(sig.return_type_id)
        .map(|t| t.to_c_string(&result.type_registry))
        .unwrap_or_else(|| "void".to_string());
    assert_eq!(return_type_str, "Point");
    assert_eq!(sig.parameters.len(), 2);
    assert_eq!(sig.to_string(&result.type_registry), "Point create_point(int x, int y)");
}

#[test]
/// test nested struct types signature
fn test_nested_struct_signature() {
    let path = PathBuf::from("test_c/testlib.o");
    let analyzer = DwarfAnalyzer::from_file(&path).expect("fail to load test library");
    let result = analyzer
        .extract_analysis(false)
        .expect("fail to extract analysis");

    let sig = result.signatures
        .iter()
        .find(|s| s.name == "create_bounding_box")
        .expect("create_bounding_box not found");

    let return_type_str = result.type_registry.get_type(sig.return_type_id)
        .map(|t| t.to_c_string(&result.type_registry))
        .unwrap_or_else(|| "void".to_string());
    assert_eq!(return_type_str, "BoundingBox");
    assert_eq!(sig.parameters.len(), 2);
}

#[test]
/// test opaque pointer types signature
fn test_opaque_pointer_signature() {
    let path = PathBuf::from("test_c/testlib.o");
    let analyzer = DwarfAnalyzer::from_file(&path).expect("fail to load test library");
    let result = analyzer
        .extract_analysis(false)
        .expect("fail to extract analysis");

    let sig = result.signatures
        .iter()
        .find(|s| s.name == "init_state")
        .expect("init_state not found");

    let return_type_str = result.type_registry.get_type(sig.return_type_id)
        .map(|t| t.to_c_string(&result.type_registry))
        .unwrap_or_else(|| "void".to_string());
    assert!(return_type_str.contains("InternalState") && return_type_str.contains("*"));
    assert_eq!(sig.parameters.len(), 0);
}

#[test]
/// test enum types signature
fn test_enum_types_signature() {
    let path = PathBuf::from("test_c/testlib.o");
    let analyzer = DwarfAnalyzer::from_file(&path).expect("fail to load test library");
    let result = analyzer
        .extract_analysis(false)
        .expect("fail to extract analysis");

    let sig = result.signatures
        .iter()
        .find(|s| s.name == "get_status")
        .expect("get_status not found");

    let return_type_str = result.type_registry.get_type(sig.return_type_id)
        .map(|t| t.to_c_string(&result.type_registry))
        .unwrap_or_else(|| "void".to_string());
    assert_eq!(return_type_str, "Status");
}

#[test]
/// test enum types signature
fn test_union_types_signature() {
    let path = PathBuf::from("test_c/testlib.o");
    let analyzer = DwarfAnalyzer::from_file(&path).expect("fail to load test library");
    let result = analyzer
        .extract_analysis(false)
        .expect("fail to extract analysis");

    let sig = result.signatures
        .iter()
        .find(|s| s.name == "create_data_union")
        .expect("create_data_union not found");

    let return_type_str = result.type_registry.get_type(sig.return_type_id)
        .map(|t| t.to_c_string(&result.type_registry))
        .unwrap_or_else(|| "void".to_string());
    assert_eq!(return_type_str, "DataUnion");
}

#[test]
/// test double pointer types signature
fn test_double_pointer_signature() {
    let path = PathBuf::from("test_c/testlib.o");
    let analyzer = DwarfAnalyzer::from_file(&path).expect("fail to load test library");
    let result = analyzer
        .extract_analysis(false)
        .expect("fail to extract analysis");

    let sig = result.signatures
        .iter()
        .find(|s| s.name == "allocate_matrix")
        .expect("allocate_matrix not found");

    let param0_type = result.type_registry.get_type(sig.parameters[0].type_id)
        .map(|t| t.to_c_string(&result.type_registry))
        .unwrap_or_else(|| "void".to_string());
    assert!(param0_type.contains("int**"));
}

#[test]
/// test double pointer types signature
fn test_variadic_function_signature() {
    let path = PathBuf::from("test_c/testlib.o");
    let analyzer = DwarfAnalyzer::from_file(&path).expect("fail to load test library");
    let result = analyzer
        .extract_analysis(false)
        .expect("fail to extract analysis");

    let sig = result.signatures
        .iter()
        .find(|s| s.name == "sum_varargs")
        .expect("sum_varargs not found");

    assert_eq!(sig.is_variadic, true);
    assert!(sig.to_string(&result.type_registry).contains("..."));
}

#[test]
/// test double pointer types signature
fn test_complex_function_signature() {
    let path = PathBuf::from("test_c/testlib.o");
    let analyzer = DwarfAnalyzer::from_file(&path).expect("fail to load test library");
    let result = analyzer
        .extract_analysis(false)
        .expect("fail to extract analysis");

    let sig = result.signatures
        .iter()
        .find(|s| s.name == "complex_function")
        .expect("complex_function not found");

    let return_type_str = result.type_registry.get_type(sig.return_type_id)
        .map(|t| t.to_c_string(&result.type_registry))
        .unwrap_or_else(|| "void".to_string());
    assert_eq!(return_type_str, "void");
    assert_eq!(sig.parameters.len(), 5);
    // Verify it has the expected complex parameter types
    assert!(
        sig.parameters
            .iter()
            .any(|p| {
                result.type_registry.get_type(p.type_id)
                    .map(|t| t.to_c_string(&result.type_registry).contains("const char*"))
                    .unwrap_or(false)
            })
    );
    assert!(
        sig.parameters
            .iter()
            .any(|p| {
                result.type_registry.get_type(p.type_id)
                    .map(|t| t.to_c_string(&result.type_registry).contains("Point*"))
                    .unwrap_or(false)
            })
    );
    assert!(
        sig.parameters
            .iter()
            .any(|p| {
                result.type_registry.get_type(p.type_id)
                    .map(|t| t.to_c_string(&result.type_registry).contains("Rectangle"))
                    .unwrap_or(false)
            })
    );
    assert!(
        sig.parameters
            .iter()
            .any(|p| {
                result.type_registry.get_type(p.type_id)
                    .map(|t| t.to_c_string(&result.type_registry).contains("Status*"))
                    .unwrap_or(false)
            })
    );
}

#[test]
/// test function pointer parameter signature
fn test_function_pointer_parameter_signature() {
    let path = PathBuf::from("test_c/testlib.o");
    let analyzer = DwarfAnalyzer::from_file(&path).expect("fail to load test library");
    let result = analyzer
        .extract_analysis(false)
        .expect("fail to extract analysis");

    let sig = result.signatures
        .iter()
        .find(|s| s.name == "register_callback")
        .expect("register_callback not found");

    assert_eq!(sig.parameters.len(), 2);
    // Should have Callback function pointer parameter
    assert!(sig.parameters.iter().any(|p| {
        result.type_registry.get_type(p.type_id)
            .map(|t| t.to_c_string(&result.type_registry) == "Callback")
            .unwrap_or(false)
    }));
}
