# dwarffi

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

### Supported platforms

+ linux x86
+ macos x86
+ macos arm64

that's it, windows support is too difficult since I do not have a windows PC to
test on. 

### repository structure

--> [`dwarffi`](./dwarffi) - library. Parsing logic, type extraction, function extraction.
    not an executable, but can be included in other (rust) projects as a static 
    library.
    
--> [`dwarffi-js`](./dwarffi-js) - CLI tool (Rust) installable via `cargo install`. Also, NPM
    package, released via cargo-dist.

dwarffi-js includes logic for generating bindings in Javascript using [ref-struct-di](https://github.com/node-ffi-napi/ref-struct-di), [ref-union-di](https://github.com/node-ffi-napi/ref-union-di), and [ref-napi](https://github.com/node-ffi-napi/ref-napi).
