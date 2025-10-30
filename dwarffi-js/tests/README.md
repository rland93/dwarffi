## integration Tests

end-to-end integration tests for the `dwarffi-js` Koffi bindings generator. at present, only works on macos.

- builds the C test library (`test_c/libtestlib.dylib`)
- generates js bindings using the `dwarffi-js` CLI
- runs a nodejs test suite that exercises the generated bindings
- parses [TAP](https://node-tap.org/) output and routes results through rust logger
- verifies that all tests pass in cargo test

### run the tests

```bash
cargo test --package dwarffi-js
```

with more verbose logging passed through:

```bash
# info
RUST_LOG=info cargo test --package dwarffi-js -- --nocapture
# debug (includes TAP)
RUST_LOG=debug cargo test --package dwarffi-js -- --nocapture
```

### dependencies

| dependency | required? | version                 | fallback                  |
|------------|-----------|-------------------------|---------------------------|
| node       | Yes       | ≥18 (≥20 recommended)   | Test skipped with warning |
| npm        | Yes       | Any (bundled with node) | Test panics               |
| make       | Yes       | Any                     | Test panics               |
| C compiler | Yes       | Any (gcc/clang)         | Build fails               |
| cargo      | Yes       | Any                     | Test fails                |

### logging:

nodejs runs tests with `--test --test-reporter=tap`

cargo test parses TAP output

test results are routed rust logging:
   - `info!()` for passing tests (✓)
   - `error!()` for failing tests (✗)
   - `debug!()` for TAP comments and diagnostics

## debugging failed integration tests

look for error messages like:
```
[ERROR] Temp directory preserved at: "/var/folders/.../T/.tmpXYZ"
[ERROR] Inspect generated bindings: /var/folders/.../T/.tmpXYZ/bindings.js
```

you can manually inspect the generated bindings and re-run the nodejs tests:
```bash
cd /var/folders/.../T/.tmpXYZ
node test.mjs
```
