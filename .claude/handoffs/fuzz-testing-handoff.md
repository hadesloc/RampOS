# Fuzz Testing Infrastructure

The fuzz testing infrastructure for `ramp-compliance` has been set up successfully.

## Directory Structure
- `crates/ramp-compliance/fuzz/` - Fuzzing workspace
- `crates/ramp-compliance/fuzz/fuzz_targets/` - Fuzz targets
- `crates/ramp-compliance/fuzz/Cargo.toml` - Fuzzing configuration

## Fuzz Targets
- `rule_parser_target`: Fuzzes the `RuleParser::parse_json` and `RuleParser::parse` methods with arbitrary byte input.

## Running Fuzz Tests
To run the fuzz tests (requires nightly Rust and cargo-fuzz):

```bash
cd crates/ramp-compliance/fuzz
cargo +nightly fuzz run rule_parser_target
```

## Setup Notes
- Created `fuzz/` directory manually as `cargo fuzz` command was not available.
- Added `ramp-compliance-fuzz` to workspace exclusions in root `Cargo.toml`.
- Configured dependencies in `fuzz/Cargo.toml`.
