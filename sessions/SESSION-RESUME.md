# Session — Current State

> Living document. Updated throughout the session, not after.

**Date:** 2026-07-13
**Branch:** N/A (not a git repo yet)
**Last Updated:** 2026-07-13 ~10:30

---

## What's Done (Recent)

### Jul 13 — Qualifier Phase 1: Build

#### Project Scaffolding
- `Cargo.toml` — Rust binary, 6 deps (clap, serde, serde_json, chrono, anyhow, glob, regex)
- 12 source modules + 1 HTML template

#### Source Modules

| File | Lines | Purpose |
|------|-------|---------|
| `main.rs` | 218 | CLI entry, clap args, orchestration flow |
| `checker.rs` | 42 | Checker trait + registry (`get_checker`) |
| `detect.rs` | 35 | Auto-detect fallow/vitest from project signals |
| `fallow.rs` | 145 | FallowChecker — runs `fallow --format json`, parses JSON |
| `vitest.rs` | 230 | VitestChecker — runs `vitest run --coverage`, reads coverage files |
| `metrics.rs` | 95 | UnifiedMetrics struct + MetricValue enum + merge logic |
| `history.rs` | 65 | qualifier.json load/append/save |
| `chart.rs` | 18 | Generate quality.html from embedded template |
| `compat_chart.rs` | 55 | Backward compat: update fallow-chart.html via regex |
| `compat_md.rs` | 80 | Backward compat: update fallow-progress.md via regex |
| `summary.rs` | 90 | CLI summary table print |

#### Template
- `templates/quality.html` — Standalone HTML dashboard with:
  - Summary cards (MI, Dead%, Dup%, Stmt%, Branch%, Func%, Max CRAP, Tests) with delta vs previous run
  - 8 canvas charts (MI, Dead, Dup, Stmt, Branch, Func, CRAP, Lines)
  - Delta table (current vs previous)
  - Commentary timeline (reverse chronological)
  - Collapsible raw JSON

#### Cross-Platform Fixes
- `run_npx()` helper in fallow.rs and vitest.rs — tries `node_modules/.bin/<pkg>.cmd` (Windows) or `node_modules/.bin/<pkg>` (Unix) first, falls back to `npx.cmd`/`npx`
- `format_comma()` in summary.rs and compat_md.rs — Rust doesn't support `{:,}` formatting
- Regex capture groups use closure callbacks (Rust's `format!` doesn't support `${1}`)

#### Bugs Fixed
- Branch coverage double-counting in Istanbul parser (181% → correct)
- Markdown regex needed `(?m)` multiline flag
- `clone_groups` missing from compat_md format string (18 placeholders, 17 args)
- `args.note` moved multiple times — resolved with early clone

#### Build Status
- **Compiles clean** — 1 warning (unused `name()` on Checker trait, kept for future)
- Release binary: `target/release/qualifier.exe`

#### Test Results
- **FileBitch (native Windows):** Fallow ✅ (LOC 5277, MI 91.5, Dead 15.8%, Dup 11.8%, Max CRAP 210)
- **PropertyShop (Docker):** Fallow ✅ (LOC 21201, MI 93.6, Dead 0%, Dup 7%, Max CRAP 992). Vitest fails (PostgreSQL not available on host)
- **Compat layer:** fallow-chart.html update ✅. fallow-progress.md update ✅ (after multiline regex fix)

---

## What's In Progress

- **Qualifier Phase 1** — Core pipeline complete, needs final validation run
- **Coverage parsing** — Istanbul fallback works but branch coverage needs verification against real data

## What's Next (Prioritized)

1. **Final validation** — Run qualifier against FileBitch (clean state), verify all 4 outputs (qualifier.json, quality.html, fallow-chart.html, fallow-progress.md)
2. **Coverage fix** — Verify Istanbul branch coverage calculation against known-good data
3. **Security checker** — Add placeholder for future security scanning (noted in design doc)
4. **`qualifier init`** — Bootstrap new projects with fallow + vitest (see `qualifier-questions.md`)
5. **Git init** — Initialize CommonTools as a git repo
6. **Tests** — Unit tests for fallow JSON parsing, Istanbul coverage parsing, metrics merge

## Current Blockers

- **None** — Build is green, waiting for validation run

## Key Gotchas

- **Windows npx:** Use `npx.cmd` on Windows, `npx` on Unix. Prefer `node_modules/.bin/<pkg>.cmd` direct path
- **Rust formatting:** No `{:,}` for comma-separated numbers. No `${1}` in format strings (use regex capture closures)
- **Regex multiline:** Need `(?m)` flag for `^` to match after newlines
- **Istanbul coverage:** `coverage-final.json` is a map of file paths → coverage data. `s` = statement hits, `f` = function hits, `b` = branch hits (array per branch group)
- **Coverage-summary.json:** Preferred source (vitest v3+). Has `total.statements.pct` directly
- **Fallow JSON:** `health.vital_signs` for metrics, `check.summary` for counts, `dupes.stats` for duplication, `health.file_scores` for per-file CRAP
- **Compat mode:** Updates existing fallow-chart.html and fallow-progress.md alongside new qualifier.json and quality.html

## Architecture

```
qualifier (Rust binary)
  ├── detect.rs       → scan project root for .fallowrc.json, vitest.config.*
  ├── fallow.rs       → npx fallow --format json → parse JSON
  ├── vitest.rs       → npx vitest run --coverage → read coverage/coverage-*.json
  ├── metrics.rs      → merge into UnifiedMetrics
  ├── history.rs      → append run to qualifier.json
  ├── chart.rs        → generate quality.html from template
  ├── compat_chart.rs → update fallow-chart.html (regex)
  ├── compat_md.rs    → update fallow-progress.md (regex)
  └── summary.rs      → print CLI table
```

## Distribution

```bash
# bashrc
alias qualifier="E:/projects/CommonTools/target/release/qualifier"
```

## Environment

- **Platform:** Windows 11 Home
- **Shell:** PowerShell (primary); Bash tool available
- **Rust:** stable-x86_64-pc-windows-msvc
- **Model:** Qwen3.6-27B-UD-Q4_K_XL (local)

---
