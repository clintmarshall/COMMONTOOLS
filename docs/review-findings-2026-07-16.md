# Review Findings — qualifier

**Date:** 2026-07-16
**Scope:** Full codebase review (13 source files)
**Status:** Pre-implementation, no fixes applied yet

---

## Confirmed Findings

### 1. `clippy.rs` — Bypasses `exec::run_command` (HIGH)

**Location:** `clippy.rs:21-24`

Clippy calls `Command::new("cargo")` directly, bypassing `exec::run_command()`. The project mandate (CLAUDE.md) is explicit: _"Checkers call `exec::run_command()` — never invoke `Command` directly."_

**Impact:** Clippy always runs on the host, even when the project uses Docker. If a Rust project runs in a container, clippy runs against the wrong toolchain or fails silently. Fallow and vitest checkers both use `exec::run_command()` correctly.

**Fix:** Use `exec::run_command(project_dir, "cargo", &["clippy", "--message-format", "json", "--"])`.

---

### 2. `clippy.rs` — Double-counts category metrics (HIGH)

**Location:** `clippy.rs:79-114`

Lines 79-93 categorize diagnostics by `span.category`, then lines 96-114 do the same by scanning `children[].message` for category strings. A single diagnostic with both a span category AND a child message mentioning the category gets counted **twice** in the same bucket.

**Example:** A `clippy::complexity` lint with `category: "clippy::complexity"` on a span AND a child message `"this is a clippy::complexity lint"` increments `complexity_issues` twice.

**Fix:** Pick one source of truth (spans are more reliable) and remove the children scan.

---

### 3. `vitest.rs` — Istanbul fallback statement/line misalignment (MEDIUM)

**Location:** `vitest.rs:225-237`

`parse_istanbul_coverage()` counts `total_stmt` from `statementMap.len()` but `covered_stmt` from the `s` hit map. The `total_lines` comes from `s.len()` which may not equal `statementMap.len()` — uncovered statements with 0 hits may be omitted from `s`, causing the total and covered counts to be misaligned.

CLAUDE.md notes: _"Original code double-counted statements (statementMap + s hits)"_ — the fix is incomplete.

**Fix:** Use `statementMap.len()` for both statement and line totals. Count covered from `s` keys with hits > 0.

---

### 4. `compat_*.rs` — Regex-based HTML/MD injection (MEDIUM)

**Location:** `compat_chart.rs:32`, `compat_md.rs:77`

Uses regex to find insertion points in existing files:
- `compat_chart.rs`: `r"(note: '[^']*' },\n)(\];)"` — fragile if template formatting changes or a note contains `'];`
- `compat_md.rs`: Regex for table separator line — breaks if markdown structure changes

**Fix:** Use a marker comment in the template (e.g., `<!-- QUALIFIER_DATA -->`) or parse the embedded data structure properly.

---

### 5. `docker.rs` — Assumes `sh` exists in container (LOW)

**Location:** `docker.rs:78`

`binary_in_container` runs `sh -c "test -f ..."` inside the container. Distroless images, Windows Nano Server containers, or minimal Alpine images without `sh` will fail. The check returns `false`, causing a fallback to host execution — which may also fail.

**Fix:** Use `ls node_modules/.bin/<binary>` or document the `sh` requirement.

---

## Plausible Findings

### 6. `main.rs` — No failure distinction in chart data (LOW)

**Location:** `main.rs:108-117`

When a checker fails, the run is saved with empty metrics. The chart shows a partial data point that looks like metrics dropped to zero rather than "not collected."

**Consider:** Add a `status: Ok | Err` field to `CheckerResult` so the chart can render "no data."

---

### 7. `metrics.rs` — Flat struct scales poorly (LOW)

**Location:** `metrics.rs:32-68`

25+ fields, each new metric requires touching the struct, `build_unified()`, and every consumer.

**Consider:** Add `extra: BTreeMap<String, MetricValue>` with `#[serde(flatten)]` as a catch-all.

---

### 8. `detect.rs` — Over-eager clippy detection (LOW)

**Location:** `detect.rs:28-30`

Any `Cargo.toml` triggers clippy, including workspace roots, consumed libraries, or the qualifier's own `Cargo.toml`. Running qualifier from within CommonTools detects itself.

**Consider:** Verify `cargo clippy --version` succeeds, or exclude the qualifier's own directory.

---

### 9. `chart.rs` — Unescaped HTML injection (LOW)

**Location:** `chart.rs:13`, `compat_chart.rs:24`

Raw JSON injected into HTML via string replacement. User notes (free text) containing `<script>` or other HTML-breaking content produce malformed HTML. Only single quotes are escaped in `compat_chart.rs`.

**Fix:** HTML-escape note strings before injection.

---

### 10. `checker.rs` — Unused `name` method (TRIVIAL)

**Location:** `checker.rs:14`

Compiler warning. `get_checker()` returns `Box<dyn Checker>` but never calls `.name()` — the name is known from the input string.

**Fix:** Remove from trait or use it in the runner loop.

---

## Summary

| # | File | Severity | Category |
|---|------|----------|----------|
| 1 | `clippy.rs` | **High** | Bypasses `exec::run_command` — breaks Docker execution |
| 2 | `clippy.rs` | **High** | Double-counts category metrics |
| 3 | `vitest.rs` | **Medium** | Istanbul parser — statement/line count misalignment |
| 4 | `compat_*.rs` | **Medium** | Regex injection into HTML/MD — fragile |
| 5 | `docker.rs` | **Low** | Assumes `sh` in container |
| 6 | `main.rs` | **Low** | No failure distinction in chart data |
| 7 | `metrics.rs` | **Low** | Flat struct scales poorly |
| 8 | `detect.rs` | **Low** | Over-eager clippy detection |
| 9 | `chart.rs` | **Low** | Unescaped HTML injection |
| 10 | `checker.rs` | **Trivial** | Dead code warning |
