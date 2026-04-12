---
globs: crates/ether-macros/**
---

# ether-macros rules

## Patterns

- Use `syn::DeriveInput` for all derive macros
- Generate code via `quote!` — never string-concatenate Rust source
- Emit `compile_error!()` for unsupported inputs (e.g. unions, unnamed fields where named required)

## Error reporting

- Use `syn::Error` and `proc_macro2::Span` for precise error locations
- Never panic in proc macros — always return a `TokenStream` with `compile_error!`
- Test error messages with `trybuild` compile-fail tests

## Testing

- Compile-pass tests: `trybuild::TestCases::new().pass("tests/pass/*.rs")`
- Compile-fail tests: `trybuild::TestCases::new().compile_fail("tests/fail/*.rs")`
- Each derive macro needs at least one pass test (struct with named fields) and one fail test (enum or union)
