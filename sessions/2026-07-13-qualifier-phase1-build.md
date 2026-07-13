# Session — Qualifier Phase 1 Build

**Date:** 2026-07-13
**Goal:** Build the qualifier binary to replace the clunky `update-quality.sh` + `update-quality.mjs` pipeline

---

## Context

PropertyShop and FileBitch use a bash script (`update-quality.sh`) that:
1. Runs `npx fallow --format json` → `/tmp/fallow.json`
2. Runs `npx vitest run --coverage` → parses stdout text
3. Runs `node update-quality.mjs` → regex-injects rows into `fallow-chart.html` and `fallow-progress.md`

Problems: POSIX-only, `/tmp/` paths, regex on HTML, text parsing of vitest output, no unified schema.

## Design Decisions

- **Rust** (not Python) — user wanted speed, cross-platform, single binary
- **Coexist mode** — update both new artifacts (qualifier.json, quality.html) AND existing ones (fallow-chart.html, fallow-progress.md)
- **Direct JSON parsing** — fallow JSON output directly, Istanbul coverage files directly (no text regex)
- **Distribution** — alias in bashrc for now
- **Security** — reserved in UnifiedMetrics struct, not implemented yet

## Implementation

### Modules Built

| Module | Purpose | Key Details |
|--------|---------|-------------|
| `main.rs` | CLI + orchestration | clap args, 12-step pipeline |
| `checker.rs` | Checker trait | `name()`, `can_run()`, `run()` |
| `detect.rs` | Auto-detection | `.fallowrc.json` → fallow, `vitest.config.*` → vitest |
| `fallow.rs` | FallowChecker | Parses `health.vital_signs`, `check.summary`, `dupes.stats`, `health.file_scores` |
| `vitest.rs` | VitestChecker | Reads `coverage-summary.json` (preferred) or `coverage-final.json` (Istanbul fallback) |
| `metrics.rs` | Unified schema | `MetricValue` enum, `UnifiedMetrics` struct, merge logic |
| `history.rs` | qualifier.json | Load/append/save with version tracking |
| `chart.rs` | quality.html | Template with `__RUNS_DATA_PLACEHOLDER__` replaced by JSON |
| `compat_chart.rs` | Backward compat | Regex append to fallow-chart.html data array |
| `compat_md.rs` | Backward compat | Regex insert into fallow-progress.md table |
| `summary.rs` | CLI output | Compact table with LOC, MI, Dead%, Dup%, Coverage, CRAP, Tests |

### Bugs Encountered and Fixed

1. **Rust format strings** — `${1}` not valid in `format!`, used regex capture closures instead
2. **Comma formatting** — `{:,}` not supported in Rust, wrote `format_comma()` helper
3. **Branch coverage double-count** — Istanbul parser incremented `covered_branch` twice per hit
4. **Regex multiline** — `^` anchor needs `(?m)` flag to match after newlines
5. **Missing format arg** — `clone_groups` was in the struct but not in the format string
6. **Moved values** — `args.note` consumed multiple times, fixed with early clone
7. **npx on Windows** — `npx` not found, added `run_npx()` helper with `node_modules/.bin/<pkg>.cmd` fallback

### Test Results

**FileBitch (native Windows):**
- Fallow: LOC 5277, MI 91.5, Dead 15.8%, Dup 11.8%, Max CRAP 210 ✅
- Vitest: Coverage 0% stmt (no coverage-v8 package), 89% func (from Istanbul fallback)
- Compat: fallow-chart.html ✅, fallow-progress.md ✅ (after multiline fix)

**PropertyShop (Docker):**
- Fallow: LOC 21201, MI 93.6, Dead 0%, Dup 7%, Max CRAP 992 ✅
- Vitest: Fails (PostgreSQL not available on host) — expected, runs in Docker

## Files Changed

- `Cargo.toml` — New
- `src/main.rs` — New
- `src/checker.rs` — New
- `src/detect.rs` — New
- `src/fallow.rs` — New
- `src/vitest.rs` — New
- `src/metrics.rs` — New
- `src/history.rs` — New
- `src/chart.rs` — New
- `src/compat_chart.rs` — New
- `src/compat_md.rs` — New
- `src/summary.rs` — New
- `templates/quality.html` — New
- `sessions/SESSION-RESUME.md` — New
- `sessions/.resume-prompt.md` — New
- `sessions/2026-07-13-qualifier-phase1-build.md` — This file
- `qualifier-questions.md` — New (brainstormed questions)

## Next Steps

1. Final validation run (clean state)
2. Coverage verification against known-good data
3. Git init
4. Security checker placeholder
5. `qualifier init` design
6. Unit tests
