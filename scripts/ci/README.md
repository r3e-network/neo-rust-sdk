# CI Scripts

This directory contains scripts to run all CI checks locally before pushing to GitHub.

## Usage

From the project root directory, run:

```bash
./scripts/ci/run-all-ci-checks.sh
```

This will run all CI checks and report any failures.

## Individual Scripts

You can also run individual checks:

- `01-format-check.sh` - Checks Rust code formatting
- `02-clippy-check.sh` - Runs Clippy linting
- `03-rust-tests.sh` - Runs Rust tests (excluding neo-gui)
- `04-benchmarks.sh` - Runs benchmarks
- `05-documentation.sh` - Builds documentation
- `06-security-audit.sh` - Runs security audits
- `07-release-check.sh` - Checks if release is ready
- `08-neo-gui-tests.sh` - Runs neo-gui frontend tests

## Requirements

- Rust toolchain with `cargo`, `rustfmt`, and `clippy`
- Node.js and npm (for neo-gui tests)
- `cargo-audit` (will be installed automatically if missing)

## Notes

- Scripts automatically exclude `neo-gui` from Rust operations to avoid GTK dependency issues
- All scripts restore the workspace to its original state after running
- The master script will continue running all checks even if some fail
- Exit code is 0 only if all checks pass