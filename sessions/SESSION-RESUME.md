# Session — Current State

> Living document. Updated throughout the session, not after.

**Date:** 2026-07-13
**Branch:** main
**Last Updated:** 2026-07-13 ~13:00

---

## What's Done (Recent)

### Jul 13 — Qualifier Phase 1: Build

#### Project Scaffolding
- `Cargo.toml` — Rust binary, 6 deps (clap, serde, serde_json, chrono, anyhow, glob, regex)
- 12 source modules + 1 HTML template

#### Source Modules

| File | Lines | Purpose |
|------|-------|---------|
| `main.rs` | ~250 | CLI entry, clap args, orchestration flow, auto output dir |
| `checker.rs` | 42 | Checker trait + registry (`get_checker`) |
| `detect.rs` | 35 | Auto-detect fallow/vitest from project signals |
| `docker.rs` | ~100 | Docker detection: compose file, running services, binary check |
| `exec.rs` | ~50 | Unified command runner (Docker first, host fallback) |
| `fallow.rs` | ~145 | FallowChecker — uses `exec::run_command()`, parses JSON |
| `vitest.rs` | ~280 | VitestChecker — uses `exec::run_command()`, parses coverage (Clover/Istanbul) |
| `metrics.rs` | 95 | UnifiedMetrics struct + MetricValue enum + merge logic |
| `history.rs` | 65 | qualifier.json load/append/save |
| `chart.rs` | 18 | Generate quality.html from embedded template |
| `compat_chart.rs` | 55 | Backward compat: update fallow-chart.html via regex |
| `compat_md.rs` | 80 | Backward compat: update fallow-progress.md via regex |
| `summary.rs` | 90 | CLI summary table print |

#### Template
- `templates/quality.html` — Standalone HTML dashboard with:
  - Overall quality score (green/amber/red) next to title
  - Summary cards with traffic light colors and delta vs previous run
  - 8 canvas charts with traffic light line colors (no dots, clean lines)
  - Charts reordered: higher-is-better on left, lower-is-better on right
  - Hover tooltips with plain English explanations
  - Run history table (latest first, color-coded deltas)
  - Collapsible raw JSON

#### Cross-Platform Fixes
- `exec::run_command()` — unified command runner: Docker container first (if binary installed there), then host `node_modules/.bin/<pkg>.cmd` (Windows) or `node_modules/.bin/<pkg>` (Unix), then `npx.cmd`/`npx`
- `format_comma()` in summary.rs and compat_md.rs — Rust doesn't support `{:,}` formatting
- Regex capture groups use closure callbacks (Rust's `format!` doesn't support `${1}`)

#### Bugs Fixed
- Branch coverage double-counting in Istanbul parser (181% → correct)
- Markdown regex needed `(?m)` multiline flag
- `clone_groups` missing from compat_md format string (18 placeholders, 17 args)
- `args.note` moved multiple times — resolved with early clone
- Istanbul parser double-counted statements (statementMap + s hits) — fixed
- Charts drew "no data" before layout — wrapped in `requestAnimationFrame`
- Coverage was null for Docker projects — added Docker auto-detection + Clover XML parsing

#### Build Status
- **Compiles clean** — 1 warning (unused `name()` on Checker trait, kept for future)
- Release binary: `target/release/qualifier.exe`

#### Test Results
- **FileBitch (native Windows):** Fallow ✅ (LOC 5277, MI 91.5, Dead 15.8%, Dup 11.8%, Max CRAP 210)
- **PropertyShop (Docker):** Fallow ✅ (runs on host), Vitest ✅ (runs in container, PostgreSQL available)
  - LOC 22,067, MI 93.6, Dead 0%, Dup 7%, Stmt 77.8%, Branch 66.2%, Func 87.1%, Max CRAP 992
  - Score: 71% amber
- **Compat layer:** fallow-chart.html update ✅. fallow-progress.md update ✅

---

## What's In Progress

- **Qualifier Phase 2** — Docker support, dashboard polish, auto output dir (done Jul 13 ~13:00)

## What's Next (Prioritized)

1. **Score button on site UI** — Embed quality score badge in PropertyShop admin header (discussed, no decision)
2. **Security checker** — Add placeholder for future security scanning (noted in design doc)
3. **`qualifier init`** — Bootstrap new projects with fallow + vitest (see `qualifier-questions.md`)
4. **Tests** — Unit tests for fallow JSON parsing, Istanbul coverage parsing, metrics merge

## Current Blockers

- **None** — Build is green, all features working

## Key Gotchas

- **Windows npx:** Use `npx.cmd` on Windows, `npx` on Unix. Prefer `node_modules/.bin/<pkg>.cmd` direct path
- **Rust formatting:** No `{:,}` for comma-separated numbers. No `${1}` in format strings (use regex capture closures)
- **Regex multiline:** Need `(?m)` flag for `^` to match after newlines
- **Istanbul coverage:** `coverage-final.json` is a map of file paths → coverage data. `s` = statement hits, `f` = function hits, `b` = branch hits (array per branch group)
- **Coverage-summary.json:** Preferred source (vitest v3+). Has `total.statements.pct` directly
- **Clover XML:** v8 provider produces `clover.xml` with `<metrics>` tag (not `<coverage>`). Parse `statements`, `coveredstatements`, `conditionals`, `coveredconditionals`, `methods`, `coveredmethods`
- **Fallow JSON:** `health.vital_signs` for metrics, `check.summary` for counts, `dupes.stats` for duplication, `health.file_scores` for per-file CRAP
- **Compat mode:** Updates existing fallow-chart.html and fallow-progress.md alongside new qualifier.json and quality.html
- **Docker detection:** Check `docker-compose.yml` exists, find running service, check binary in container's `node_modules/.bin/` before using container
- **Output dir:** Next.js/Vite → `public/` (auto-served). `qualifier.json` always in project root (data, not served)

## Architecture

```
qualifier (Rust binary)
  ├── detect.rs       → scan project root for .fallowrc.json, vitest.config.*
  ├── docker.rs       → detect docker-compose, find running service, check binary in container
  ├── exec.rs         → unified command runner (Docker first, host fallback)
  ├── fallow.rs       → exec::run_command("fallow", ["--format", "json"]) → parse JSON
  ├── vitest.rs       → exec::run_command("vitest", ["run", "--coverage"]) → parse coverage
  ├── metrics.rs      → merge into UnifiedMetrics
  ├── history.rs      → append run to qualifier.json
  ├── chart.rs        → generate quality.html from template
  ├── compat_chart.rs → update fallow-chart.html (regex)
  ├── compat_md.rs    → update fallow-progress.md (regex)
  └── summary.rs      → print CLI table
```

## Distribution

```bash
# bashrc (Windows host)
alias qualifier="E:/projects/CommonTools/target/release/qualifier"
```

## Running in Docker Projects (PropertyShop)

**Qualifier auto-detects Docker.** Just run from host:
```bash
qualifier --project-dir E:/projects/PropertyShop --note "Phase 4M done"
```
- Checks for `docker-compose.yml` → finds running service → checks if binary exists in container
- vitest runs inside container (PostgreSQL available) → real coverage data
- fallow runs on host (not installed in container) → works via host node_modules
- Output: `qualifier.json` in project root, HTML files in `public/` (auto-served by Next.js)

**Override output location:**
```bash
qualifier --output-dir ./reports --note "custom location"
```

## Environment

- **Platform:** Windows 11 Home
- **Shell:** PowerShell (primary); Bash tool available
- **Rust:** stable-x86_64-pc-windows-msvc
- **Model:** Qwen3.6-27B-UD-Q4_K_XL (local)

---
