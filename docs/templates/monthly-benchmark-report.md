# Neo Rust SDK — Monthly Performance Report: {{MONTH}}

> Generated: {{GENERATED_UTC}}
> Benchmarks measured: {{BENCH_COUNT}}
> Source: Criterion baseline `monthly-{{MONTH}}` (see `.github/workflows/benchmark-trend.yml`)

This report is produced automatically on the 1st of each month by the
**Benchmark Trend Report** workflow. It captures the mean and p50/p95/p99
latency percentiles for every benchmark in the suite so month-over-month
performance trends and regressions are visible over time.

## Results

{{RESULTS_TABLE}}

## Acceptance KPIs (v3.3.0 Phase 5)

| KPI | Target | Status |
|-----|--------|--------|
| Single-key signing p99 | < 10 ms | _verify `crypto/sign/ecdsa_sign_prehash` p99 above_ |
| p50 month-over-month regression | < 10% increase | _compare vs previous month's report_ |
| Public-API benchmark coverage | ≥ 1 bench per public API | _tracked in `docs/BENCHMARKING.md`_ |

## How to interpret

- **Mean** is Criterion's point estimate over all samples (the primary
  regression signal used by the CI gate in `benchmark.yml`).
- **p50 / p95 / p99** are nearest-rank percentiles derived from the raw
  per-iteration timings in Criterion's `sample.json`. p99 is the tail-latency
  figure used for the signing KPI.
- Throughput-annotated benches (hashing, serialization, batch signing) also
  emit bytes/s or elements/s in the HTML report artifacts.

## Trend actions

1. Compare this month's means to the previous report (`benchmark-report-*.md`
   artifacts, retained 400 days).
2. If any operation's p50 rose > 10% vs last month, open a performance issue and
   link the two reports.
3. Attach the Criterion HTML report artifact for the affected benchmark group.

## Reproduce locally

```bash
# Full suite with HTML reports
cargo criterion

# Save this month's baseline and regenerate the report
cargo bench --benches -- --save-baseline monthly-{{MONTH}}
python3 scripts/render_monthly_report.py \
  --baseline monthly-{{MONTH}} \
  --template docs/templates/monthly-benchmark-report.md \
  --month {{MONTH}} --output benchmark-report-{{MONTH}}.md
```

See [`docs/BENCHMARKING.md`](../BENCHMARKING.md) for the complete guide.
