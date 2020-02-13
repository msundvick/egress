# Egress - barebones regression testing for Rust

Egress is a super simple regression testing framework for Rust. It
doesn't currently support much, but if all you want is to make sure
some test outputs don't change from run to run, it'll do the trick.

By default, Egress will make an `Egress.toml` config file in the same
directory as your `Cargo.toml` and an `egress` folder in the same place
to hold the artifacts it writes to disk.

## Example

```rust
let mut egress = egress!();
let artifact = egress.artifact("basic_arithmetic");

let super_complex_test_output_that_could_change_at_any_time = 1 + 1;

// using `serde::Serialize`:
artifact.insert_serialize("1 + 1 (serde)", &super_complex_test_output_that_could_change_at_any_time);

// or using `fmt::Debug`:
artifact.insert_debug("1 + 1 (fmt::Debug)", &super_complex_test_output_that_could_change_at_any_time);

// or using `fmt::Display`:
artifact.insert_display("1 + 1 (fmt::Display)", &super_complex_test_output_that_could_change_at_any_time);

// More options available; please check the docs.

egress.close().unwrap().assert_unregressed();
```

To see the artifacts produced by this example, check `egress/artifacts/rust_out/basic_arithmetic.json`.