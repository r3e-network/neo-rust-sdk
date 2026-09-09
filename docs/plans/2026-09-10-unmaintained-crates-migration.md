# Unmaintained Crates Migration Decision Document

**Task:** #118 — Plan migration for unmaintained crates flagged by Dependabot / `cargo audit`
**Date:** 2026-09-10
**Scope:** `derivative` (RUSTSEC-2024-0388), `number_prefix` (RUSTSEC-2025-0119), `paste` (RUSTSEC-2024-0436)

---

## 0. Executive Summary

| Crate | Advisory | Class | In active build graph? | How it enters | Recommended option | Effort |
|-------|----------|-------|------------------------|---------------|--------------------|--------|
| `derivative` 2.2.0 | RUSTSEC-2024-0388 | **Unmaintained** (not a vuln) | **NO — stale lock entry** | (was) `ark-ff 0.3/0.4` — no longer reachable | **A — Fix now** (`cargo update` prunes it) | ~5 min |
| `number_prefix` 0.4.0 | RUSTSEC-2025-0119 | **Unmaintained** (not a vuln) | Yes, **dev-only** | `cargo-nextest` dev-dependency → `nextest-runner` → `indicatif` | **A — Fix now** (drop the misplaced dev-dep) | ~10 min |
| `paste` 1.0.15 | RUSTSEC-2024-0436 | **Unmaintained** (not a vuln) | Yes, **transitive (proc-macro)** | `alloy` (direct dep) → `alloy-primitives` | **C — Accept risk** (track + `deny.toml` ignore) | ~5 min |

**Key framing:** All three advisories are **"unmaintained"** notices, *not* vulnerabilities. There is no known exploit path; `paste` and `derivative` are compile-time-only proc-macros and `number_prefix` never ships in the library artifact. Risk is supply-chain hygiene, not runtime security.

**Nothing in our own source code uses any of the three crates** — verified: zero matches for `#[derive(Derivative)]`, `paste!`/`paste::`, or `number_prefix` across `src/`, `neo-cli/`, `tests/`, `benches/`, `examples/`. All usage is transitive.

Two of the three (`derivative`, `number_prefix`) can be **eliminated today with zero code changes and zero API impact**. Only `paste` is a genuine long-lived transitive dependency, and it is gated behind the actively-maintained `alloy` stack.

---

## 1. Investigation Method & Evidence

Commands run (Windows PowerShell; `cargo tree -i` requires `--target all` because these crates arrive through platform-gated / proc-macro edges):

```
cargo tree -i derivative    --target all --all-features --workspace   # => "nothing to print"
cargo tree -i paste         --target all                              # => alloy-primitives -> alloy -> neo3
cargo tree -i number_prefix --target all                              # => indicatif -> nextest-runner -> cargo-nextest [dev]
cargo tree --target all --all-features --workspace | grep "ark-"      # => (empty) — no arkworks in active graph
```

Source-code usage scan (all empty):

```
Get-ChildItem -Recurse -Include *.rs -Path src,neo-cli,tests,benches,examples |
  Select-String "Derivative|paste!|paste::|number_prefix"
```

`Cargo.lock` cross-check:
- `derivative` (line 2382) is referenced only by `ark-ff 0.3.0` (L816) and `ark-ff 0.4.2` (L834).
- `ark-ff 0.5.0`/`0.6.0` already dropped `derivative` in favour of `educe`.
- **No `ark-*` crate is present in the resolved build graph** for any feature/target — the `ark-ff 0.3/0.4` entries (and therefore `derivative`) are orphaned leftovers from a previous dependency resolution (an older `alloy`/crypto stack that pulled arkworks for BLS). `cargo update` regenerates the lockfile without them.

---

## 2. Per-Crate Analysis & Options

### 2.1 `derivative` 2.2.0 — RUSTSEC-2024-0388

- **Status:** Unmaintained proc-macro. Was used to derive `Debug`/`Clone`/etc. with customisation.
- **Reality in this repo:** **Not compiled at all.** It is a stale `Cargo.lock` artifact via `ark-ff 0.3/0.4`, which are themselves unreachable. `cargo tree -i derivative --all-features --workspace --target all` prints *nothing*.
- **Direct dependency?** No. **Used in our code?** No (`derive_more = { version = "2.1.1", features = ["full"] }` is already the project's chosen derive helper — see [Cargo.toml L162](file:///d:/Git/neo-rust-sdk/Cargo.toml#L162)).

**Options**

- **A — Fix now (RECOMMENDED).** Run `cargo update` (optionally `cargo update -p ark-ff` / regenerate) so the stale `derivative` + `ark-ff 0.3/0.4` entries drop out of `Cargo.lock`. Verify with `cargo tree -i derivative --target all` → "nothing to print" and `cargo audit`/`cargo deny` no longer report RUSTSEC-2024-0388. **No source changes, no API changes.**
- **B — Defer.** No benefit; the advisory keeps firing on every audit for a crate we do not build.
- **C — Accept risk.** Unnecessary; it costs nothing to remove.

> **Note:** If, after `cargo update`, `derivative` still lingers, run `cargo update -p derivative --precise` cannot help (no newer version); instead confirm nothing pulls `ark-ff <=0.4`. Current evidence says nothing does — the lockfile simply needs regenerating.

---

### 2.2 `number_prefix` 0.4.0 — RUSTSEC-2025-0119

- **Status:** Unmaintained. Formats byte/SI number prefixes; pulled in by `indicatif` (progress bars).
- **Path:** `number_prefix` → `indicatif 0.17` → `nextest-runner 0.74` → `cargo-nextest 0.9` (**dev-dependency only**, [Cargo.toml L224](file:///d:/Git/neo-rust-sdk/Cargo.toml#L224)).
- **Direct dependency?** No. **Ships in the library?** No — dev-dependency, never in the published crate or downstream builds.
- **Root cause worth flagging:** `cargo-nextest` is a **binary test runner**, not a library. Listing it under `[dev-dependencies]` is almost certainly a mistake — it compiles a large, unused tree (`nextest-runner`, `indicatif`, `self_update`, `number_prefix`, …) into the dev graph for no benefit. Nextest is normally obtained via `cargo install cargo-nextest` / `cargo binstall` or a CI action, not a manifest dependency.

**Options**

- **A — Fix now (RECOMMENDED).** Remove the `cargo-nextest = "0.9"` line from `[dev-dependencies]`. This eliminates `number_prefix` **and** trims a substantial chunk of the dev dependency graph. If CI relies on nextest, install it as a tool (CI: `taiki-e/install-action@nextest` or `cargo install`) rather than as a manifest dependency. Verify: `cargo tree -i number_prefix --target all` → "nothing to print"; `cargo test`/nextest still run.
- **B — Defer.** Acceptable short-term (dev-only, no runtime exposure) but leaves a needless heavy dev-dep and a recurring audit warning.
- **C — Accept risk.** Legitimate given dev-only scope, but strictly worse than A since A also removes accidental bloat.

**Migration detail (Option A):**
- File touched: [Cargo.toml](file:///d:/Git/neo-rust-sdk/Cargo.toml) — delete line 224.
- API changes: none.
- CI follow-up: if any workflow invokes `cargo nextest`, ensure the runner installs the nextest binary (tool install step), independent of the manifest.

---

### 2.3 `paste` 1.0.15 — RUSTSEC-2024-0436

- **Status:** Unmaintained proc-macro (token pasting/`concat_idents`). Compile-time only; nothing at runtime.
- **Path (active):** `paste` → `alloy-primitives 1.6.1` → `alloy 2.4.1` → **`neo3` (direct dep, [Cargo.toml L197](file:///d:/Git/neo-rust-sdk/Cargo.toml#L197))**. (Its secondary `ark-ff` path is stale, per §1.)
- **Direct dependency?** No. **Used in our code?** No. It is intrinsic to the `alloy` EVM stack that powers the Neo X bridge; we cannot remove it without removing `alloy`, which is not on the table.

**Options**

- **A — Fix now.** Not feasible on our side — the fix must come from upstream `alloy-primitives` migrating off `paste`. `alloy-rs` is actively maintained and tracking this. We cannot patch a transitive proc-macro without vendoring/forking `alloy`, which is disproportionate.
- **B — Defer / monitor (secondary).** Bump `alloy` on Dependabot's schedule; a future `alloy-primitives` release is expected to drop `paste`. Re-check `cargo tree -i paste` after each `alloy` bump.
- **C — Accept risk + document (RECOMMENDED).** Add `RUSTSEC-2024-0436` to the `[advisories].ignore` list in [deny.toml](file:///d:/Git/neo-rust-sdk/deny.toml) with a tracking comment (mirroring the existing `RUSTSEC-2023-0071` entry pattern), since it is an unmaintained-only notice on a compile-time-only crate with no runtime exposure. Combine with B (monitor `alloy` upgrades to remove it naturally).

**Migration detail (Option C):**
- File touched: [deny.toml](file:///d:/Git/neo-rust-sdk/deny.toml) — append to the existing `ignore = [ ... ]` array:
  ```toml
  # RUSTSEC-2024-0436: `paste` unmaintained. Compile-time-only proc-macro pulled
  # transitively by alloy-primitives (Neo X EVM bridge). No runtime exposure.
  # Remove once alloy-primitives drops the paste dependency.
  # Tracked: https://rustsec.org/advisories/RUSTSEC-2024-0436
  "RUSTSEC-2024-0436",
  ```
- API changes: none.

---

## 3. Prioritized Action Plan

| Priority | Crate | Action | Files touched | API impact | Effort | Risk if skipped |
|----------|-------|--------|---------------|-----------|--------|-----------------|
| **P1 (do now)** | `derivative` | `cargo update` to prune stale `ark-ff`/`derivative` lock entries; verify with `cargo tree -i derivative` | `Cargo.lock` | None | ~5 min | Recurring false-positive audit noise |
| **P1 (do now)** | `number_prefix` | Remove misplaced `cargo-nextest` dev-dep; move nextest to a CI tool-install step | `Cargo.toml` (L224); CI workflow if applicable | None | ~10 min | Heavy unused dev tree + audit noise |
| **P2 (accept + monitor)** | `paste` | Ignore RUSTSEC-2024-0436 in `deny.toml` with tracking note; re-check after each `alloy` bump | `deny.toml` | None | ~5 min | None (informational only) |

**Suggested sequence (single PR):**
1. Remove `cargo-nextest` from `[dev-dependencies]`; adjust CI to install the nextest binary.
2. `cargo update` and regenerate `Cargo.lock`.
3. Add the `paste` ignore entry to `deny.toml`.
4. Run `cargo build --all-features`, `cargo test` (or nextest via installed binary), `cargo deny check advisories`, `cargo audit`.
5. Confirm `cargo tree -i derivative` and `cargo tree -i number_prefix` both print "nothing to print".

**Expected end state:** 2 of 3 advisories fully eliminated with zero code/API change; the remaining one (`paste`) is documented, risk-accepted, and gated on upstream `alloy`.

---

## 4. Verification Checklist

- [ ] `cargo tree -i derivative --target all --all-features --workspace` → *nothing to print*
- [ ] `cargo tree -i number_prefix --target all` → *nothing to print*
- [ ] `cargo tree -i paste --target all` → only `alloy-primitives` path remains (expected; risk-accepted)
- [ ] `cargo build --all-features` succeeds
- [ ] `cargo test` / `cargo nextest run` (via installed binary) succeeds
- [ ] `cargo deny check advisories` passes (RUSTSEC-2024-0388 & -2025-0119 gone; -2024-0436 ignored with note)
- [ ] `Cargo.lock` diff shows removal of `derivative`, `ark-ff 0.3/0.4`, `number_prefix`, `indicatif`, `nextest-runner`, `cargo-nextest` entries

---

## 5. Notes & Caveats

- `deny.toml` currently sets `unmaintained = "workspace"`, so cargo-deny may only flag unmaintained crates reachable from workspace members; the Dependabot/`cargo audit` report is the stricter source that surfaced all three. The plan above satisfies both.
- This document proposes changes but makes **no code edits** — it is the requested decision deliverable. Execution (the single PR in §3) should be a follow-up task once options are approved.
- All three advisories are **unmaintained** classifications, not CVEs. None of them expands the runtime attack surface of the shipped `neo3` library.
