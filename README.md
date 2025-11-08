# dwarffi

*extract a structured representation of function signatures from a dynamic library*

### how to use:

1. compile your C library with debug symbols.
2. use `__attribute__((visibility("default")))` to selectively make symbols visible.
3. run the tool:
```bash
dwarffi-js --js --functions path/to/library.dylib >> ./bindings.js
```
4. the javascript code is printed to stdout, so you can pipe it to a file like the example above.

this tool only works with dynamic C libraries, compiled with gcc or clang, on a macos or linux system. Your library MUST have DWARF debug symbols -- i.e. `-g` flag passed in. So, it may not work with pre-compiled libraries especially in cases where the source is not available for compilation.

the intended use case is for generating bindings for a program whose build/configuration system is very complex, whose compilation is expensive, or whose source code cannot be changed or annotated, or where compilation of the library and running of the library is done in a very different context.

for an example library, see the `test_c` folder and its makefile.

### Supported platforms

+ linux x86
+ macos arm64

integration tests run on linux and mac.

windows is too difficult to test because I do not
have a windows machine.

### repository structure

--> [`dwarffi`](./dwarffi) - library. Parsing logic, type extraction, function
    extraction. not an executable, but can be included in other (rust) projects
    as a static library.
    
--> [`dwarffi-js`](./dwarffi-js) - CLI tool (Rust) installable via `cargo install`. Also -- eventually -- NPM
    package, released via cargo-dist.

dwarffi-js can generate bindings in Javascript using [koffi](https://koffi.dev/)