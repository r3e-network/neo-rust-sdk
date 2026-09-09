# Benchmarking Guide

Production performance benchmarking for the Neo Rust SDK, built on
[Criterion.rs](https://bheisler.github.io/criterion.rs/book/) with
[`cargo-criterion`](https://github.com/bheisler/cargo-criterion) for statistical
significance, p95/p99 latency tracking, and CI regression gating.

This guide covers the v3.3.0 Phase 5 benchmark suite: what is measured, how to
run it locally, how the CI regression gate works, and how to profile
allocation-heavy operations with flamegraphs.

---

## Suite layout

Benchmarks live in [`benches/`](../benches). Each file is a Criterion harness
(`harness = false` in `Cargo.toml`) grouped into **micro-benchmarks** (isolated
primitives) and **macro-benchmarks** (end-to-end workflows).

| File | Groups | Focus |
|------|--------|-------|
| `crypto_benchmarks.rs` | `crypto/keygen`, `crypto/sign`, `crypto/verify`, `crypto/hash256`, `crypto/account`, `crypto/wif` | ECDSA signing/verification, hashing, key/address/WIF derivation |
| `gas_estimator_benchmarks.rs` | `gas/calculations`, `gas/script_building`, `gas/script_sizes`, `gas/opcode_emission` | Fee accuracy math, script compilation speed under load |
| `script_builder_benchmarks.rs` | `script/primitives`, `script/transaction` | Script builder primitives + full transaction assembly |
| `wallet_benchmarks.rs` | `wallet/lifecycle`, `wallet/encryption`, `wallet/decryption`, `wallet/backup_recovery` | scrypt encryption/decryption, account add, backup/recover |
| `production_benchmarks.rs` | `prod/sign_pipeline`, `prod/serialization`, `prod/batch_signing` | Cross-domain end-to-end workflows + throughput under load |

Micro vs macro is a naming/grouping convention:
- **Micro**: single primitive, generous `sample_size` (150–300) for tight CIs.
- **Macro**: full workflow, smaller `sample_size` (10–150) to bound wall time;
  scrypt-heavy wallet benches deliberately use `sample_size = 10`.

Every group sets explicit `warm_up_time` and `measurement_time` so results are
comparable across machines and across time.

---

## Running locally

### Prerequisites

```bash
# The statistical runner with HTML reports (recommended)
cargo install cargo-criterion --locked

# Optional: baseline diffing and flamegraphs
cargo install critcmp --locked
cargo install flamegraph --locked   # needs perf (Linux) or dtrace (macOS)
```

### Run everything

```bash
# Rich statistical output + HTML reports in target/criterion/**/report/index.html
cargo criterion

# Plain cargo fallback (no cargo-criterion needed)
cargo bench
```

### Run a single suite or benchmark

```bash
cargo criterion --bench crypto_benchmarks
cargo bench     --bench production_benchmarks

# Filter by benchmark id (regex): only the signing benchmark
cargo bench --bench crypto_benchmarks -- crypto/sign
```

### Compile-only (fast validation, no measurement)

```bash
cargo bench --no-run
```

### Save and compare baselines

```bash
# Save a named baseline (e.g. before a change)
cargo bench -- --save-baseline before

# ... make your change, then compare against it
cargo bench -- --baseline before

# Side-by-side table across baselines
critcmp before new
```

---

## p95 / p99 latency tracking

Criterion records the full sample distribution. For every benchmark it writes:

```
target/criterion/<group>/<bench_id>/<baseline>/estimates.json   # mean/median/slope
target/criterion/<group>/<bench_id>/<baseline>/sample.json      # raw iters + times
```

The `mean.point_estimate` in `estimates.json` is the primary regression signal.
Percentiles (p50/p95/p99) are derived from the raw per-iteration timings in
`sample.json` by [`scripts/check_bench_regression.py`](../scripts/check_bench_regression.py)
and [`scripts/render_monthly_report.py`](../scripts/render_monthly_report.py).

The HTML report (`report/index.html`) additionally visualises the PDF, the
regression line, and the confidence interval for each benchmark.

**Phase 5 KPI:** p99 latency for single-key signing (`crypto/sign/ecdsa_sign_prehash`)
must stay **< 10 ms**. In practice it measures in the tens of microseconds; the
10 ms ceiling is a hard safety bound.

---

## CI regression gate

Two workflows drive continuous performance tracking:

### `.github/workflows/benchmark.yml` — per-PR / per-push gate

On every pull request and push to `main`/`master`:

1. Checks out full history.
2. Runs the suite on the **baseline commit** (PR base, or `HEAD^` on push) and
   saves it as the `base` Criterion baseline.
3. Runs the suite on the **current revision**, saving it as `current`.
4. [`scripts/check_bench_regression.py`](../scripts/check_bench_regression.py)
   compares `current` vs `base` per benchmark and **fails the job if any
   benchmark's mean is >5% slower** (threshold configurable via the
   `regression_threshold` workflow input / `REGRESSION_THRESHOLD` env).
5. A markdown table (Δ%, p50/p95/p99, status) is written to the job summary, and
   Criterion HTML reports are uploaded as artifacts.

New benchmarks with no baseline are reported as `🆕 new` and never fail the gate.

### `.github/workflows/benchmark-trend.yml` — monthly trend report

On the 1st of every month (cron) it runs the full suite, saves a
`monthly-YYYY-MM` baseline, and renders a report from
[`docs/templates/monthly-benchmark-report.md`](templates/monthly-benchmark-report.md)
into `benchmark-report-YYYY-MM.md` (uploaded as an artifact, retained 400 days).
Trend alerting fires if p50 rises >10% month-over-month.

---

## Flamegraphs for allocation-heavy operations

Some operations are allocation- or CPU-heavy and benefit from flamegraph
profiling to find hot spots that the aggregate timings alone don't reveal:

| Operation | Why profile it | Benchmark id |
|-----------|----------------|--------------|
| Wallet scrypt encryption | scrypt is memory-hard; dominates wallet cost | `wallet/encryption/*` |
| Large script assembly | repeated `Vec` growth / `BigInt` allocation | `gas/script_sizes/*`, `prod/serialization/*` |
| Batch signing | per-signature allocation under load | `prod/batch_signing/*` |
| 1 MB hashing | buffer handling / copy costs | `crypto/hash256/1048576` |

### Generating a flamegraph

`cargo-flamegraph` runs a benchmark binary under a sampling profiler and emits an
interactive SVG:

```bash
# Linux (perf) / macOS (dtrace). --bench selects the harness; the trailing args
# are passed to Criterion so you can target one benchmark and run it in-process.
cargo flamegraph --bench wallet_benchmarks -- --bench wallet/encryption

# Output: flamegraph.svg  (open in a browser; width = time spent)
```

For allocation profiling specifically, pair a benchmark with
[`dhat`](https://docs.rs/dhat) or run under `valgrind --tool=dhat` / `heaptrack`:

```bash
# Heap allocation hot spots for the serialization workflow
heaptrack cargo bench --bench production_benchmarks -- prod/serialization
heaptrack_gui heaptrack.*.zst
```

### Reading a flamegraph

- **Width** = proportion of samples (time) spent in a frame and its children.
- Wide leaf frames are the hot spots; look for unexpected `alloc`/`memcpy`/
  `BigInt` frames under the script/serialization benches.
- Compare before/after SVGs when optimising: the target frame should shrink.

Commit notable flamegraphs to a PR description (not the repo) and link the
Criterion report artifact so reviewers can correlate the profile with the delta.

---

## Adding a new benchmark

1. Add a `fn bench_xxx(c: &mut Criterion)` to the most relevant file, using a
   `benchmark_group("<domain>/<name>")` with explicit warmup/measurement/sample
   config (copy an existing `*_group` helper).
2. Wrap inputs/outputs in `std::hint::black_box(..)` to defeat the optimiser.
3. Use `group.throughput(..)` for byte/element-scaled operations.
4. Register the function in the file's `criterion_group!`.
5. If it is a whole new file, ask the manifest owner to add a matching
   `[[bench]] name = "<file>" harness = false` entry.
6. Run `cargo bench --no-run` to confirm it compiles, then `cargo criterion
   --bench <file>` to see the report.

**Baseline documentation:** record indicative p50/p99 targets in the file's
top-of-file doc comment table (see existing files). The authoritative numbers
always live in `target/criterion/**/estimates.json` and the CI artifacts.
