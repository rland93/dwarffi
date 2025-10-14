# ffitool

*extract a structured representation of function signatures from a dynamic library*

### how to use:

1. compile your C library with debug symbols.
2. use `__attribute__((visibility("default")))` to selectively make symbols visible.
3. run the tool:
```bash
cargo run -- path/to/library.dylib
```
4. the extracted functions are printed to console.

This tool only works with dynamic C libraries, compiled with gcc or clang, on a macos or linux system. Your library MUST have DWARF debug symbols -- i.e. `-g` flag passed in. So, it may not work with pre-compiled libraries especially in cases where the source is not available for compilation.

For an example library, see the `test_c` folder and its makefile.

### supported features:

+ Primitive types: `int`, `float`, `double`, `char`
+ Fixed-width integer types: `uint8_t`, `int64_t`
+ Special types: `size_t`, `void`
+ Simple enums with explicit values (`Status`)
+ Enums with implicit values (`Color`)
+ Simple structs (`Point`, `Rectangle`)
+ Nested structs (`BoundingBox` contains `Point`)
+ Complex structs with multiple field types (`Person`)
+ Opaque types (forward declarations: `InternalState`)
+ Basic unions (`DataUnion`)
+ Single pointers (`int*`, `char*`)
+ Double pointers (`int**`)
+ Const pointers (`const char*`, `const int*`)
+ Function pointers (`Callback`, `Comparator`)
+ Void functions with no parameters
+ Functions returning various types
+ Functions with multiple parameters
+ Functions passing structs by value
+ Functions passing structs by pointer
+ Functions with mixed parameter types
+ Variadic functions
+ Array parameters (decay to pointers)
+ Exported functions marked with `__attribute__((visibility("default")))`
+ Hidden/internal functions without visibility attribute
+ Compiled with `-fvisibility=hidden` to hide by default