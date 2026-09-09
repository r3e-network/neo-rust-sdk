# Fuzz Testing Documentation

## Overview

Neo Rust SDK v3.2.0 implements comprehensive fuzz testing infrastructure to detect vulnerabilities, edge cases, and malformed input handling issues across critical components.

## Purpose

Fuzz testing randomly generates inputs to test code paths that might not be covered by traditional unit tests. This helps discover:

- **Panic conditions** that could crash the application
- **Security vulnerabilities** from unhandled edge cases
- **Memory safety issues** with malformed data
- **Parsing errors** from unexpected input formats

## Test Coverage

### 1. Script Parser Fuzzing (`tests/fuzz/script_parser_fuzz.rs`)

**What it tests:** NeoVM script parsing with arbitrary byte sequences

**Key scenarios:**
- Empty scripts
- Single-byte scripts (edge case)
- Scripts up to 65KB (max u16 length)
- All-opcode patterns
- Extremely large inputs (>1MB)

**Expected behavior:** Should always return `Result`, never panic

### 2. Cryptographic Fuzzing (`tests/fuzz/cryptographic_fuzz.rs`)

**What it tests:** Hash functions, key generation, signature validation

**Key scenarios:**
- Random input for hash functions (SHA256)
- Zero bytes and all-ones bytes patterns
- Corrupted signatures (bit-flipping attacks)
- Large data inputs (up to 1MB)
- Malformed public keys

**Expected behavior:** Handle all crypto edge cases gracefully

### 3. RPC Response Deserialization (`tests/fuzz/rpc_response_fuzz.rs`)

**What it tests:** JSON parsing of blockchain responses

**Key scenarios:**
- Arbitrary random strings as JSON
- Invalid NEP-17 balance responses
- Malformed block headers
- Deep nesting levels (100+ levels)
- Unicode and control characters
- Special escape sequences

**Expected behavior:** Failures should be represented as `Err`, not panics

## Running Tests

### Local Development

```bash
# Run all fuzz tests
cargo test --all-features --lib --test fuzz

# Run specific fuzz target
cargo test --lib script_parser_fuzz

# Run property-based tests with verbose output
cargo test --features proptest -- --nocapture
```

### CI/CD Integration

Fuzz tests run automatically:
- **Daily at 2 AM UTC** via scheduled workflow
- **On every pull request** (paths matching src/**, tests/fuzz/**)
- **On push to main/master branches**

## Configuration

### Proptest Settings

Property-based tests use these defaults:
- **Max shrink iterations:** 50 (to simplify failing inputs)
- **Max total time per test:** 60 seconds
- **Test cases per property:** 1000 random inputs

### Environment Variables

```bash
# Increase fuzz complexity
export PROPTEST_CASES=10000        # More test cases
export PROPTEST_MAX_SHRINK_ITERS=100  # Better minimization
export PROPTEST_VERBOSE=1          # Detailed output
```

## Interpreting Results

### Passing Tests ✅

```
test test_script_parsing_no_panics_with_random_data ... ok
proptest: Success! Ran 1000 tests over {random sources}
No failures observed - excellent robustness!
```

### Failing Tests ⚠️

When a test fails, you'll see:
```
thread 'test_script_parsing_no_panics_with_random_data' panicked at 'index out of bounds'
  /src/some_module.rs:42:15

proptest failed with input: [0xFF, 0x00, 0x01, ...]
Shrunk to minimal failing input: [0xFF]
```

**Next steps:**
1. Note the minimal failing input (the "shrunk" version)
2. Reproduce locally with that exact input
3. Add a regression test with the specific pattern
4. Fix the underlying issue
5. Commit both the fix and the new test

### Known Limitations

Some tests may have `continue-on-error` in CI because they're designed to handle failure states gracefully. Look for:
- Tests explicitly marked with `#[ignore]`
- Workflows with `continue-on-error: true`

These are expected behaviors where the SDK correctly rejects invalid input.

## Adding New Fuzz Tests

### Template Structure

```rust
use proptest::prelude::*;

/// Describe what scenario this test covers
#[test]
fn test_name_for_scenario() {
    proptest!(|(input in any::<InputType>())| {
        // Generate multiple random inputs
        let result = your_function(&input);
        
        // Assert properties that must always hold
        assert!(result.is_ok() || result.is_err());
        // Or check specific constraints
        assert!(computed_length < MAX_LENGTH);
    });
}
```

### Best Practices

1. **Use property-based assertions** instead of fixed values
2. **Test edge cases explicitly** (empty, zero, max values)
3. **Document expected behavior** in comments
4. **Run locally frequently** before pushing (can take minutes)
5. **Add corpus files** if you discover interesting failure patterns

## Maintenance

### Updating Dependencies

```toml
# In Cargo.toml dev-dependencies section
proptest = "1.11"           # Keep current major version
proptest-derive = "0.4"     # Property derivation
cargo-nextest = "0.9"       # Advanced test runner
```

### Performance Tuning

If tests take too long:
```bash
# Reduce test iterations locally
PROPTEST_CASES=100 cargo test --features proptest

# Skip slow tests in CI
cargo test --features proptest --skip-slow-tests
```

## Resources

- [Proptest Documentation](https://docs.rs/proptest/)
- [PropEr (Erlang property testing)](https://proprietary-testing.com/)
- [LLVM LibFuzzer](https://llvm.org/docs/LibFuzzer.html)
- AFL++ Project](https://aflplus.plus/)

---

**Version:** v3.2.0 (in development)  
**Last Updated:** September 6, 2026  
**Maintainer:** R3E Network Security Team
