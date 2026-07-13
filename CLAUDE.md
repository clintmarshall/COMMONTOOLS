# CommonTools — Agent Operating Guide

## Project Overview

**Qualifier** — a Rust CLI binary that runs quality checkers against a project and generates a dashboard. Drop it into any project, run once, get metrics + charts.

### What it does
1. **Detects** quality tools in a project (fallow, vitest, clippy)
2. **Runs** each checker, collecting metrics
3. **Generates** `quality.html` (dashboard with charts) + `qualifier.json` (run history)
4. **Updates** legacy compat files (`fallow-chart.html`, `fallow-progress.md`) if they exist

### Key Mandate
**Zero manual steps.** The tool figures out everything: where to run commands, where to put output, which coverage format to parse.

## Architecture

```
src/
├── main.rs          # CLI entry, orchestrates detect → run → write
├── detect.rs        # Scans project for .fallowrc.json, vitest.config.*, Cargo.toml
├── checker.rs       # Trait: Checker { name, can_run, run }
├── exec.rs          # Unified command runner (Docker first, host fallback)
├── docker.rs        # Docker detection: compose file, running services, binary check
├── fallow.rs        # FallowChecker — dead code, duplication, cyclomatic complexity
├── vitest.rs        # VitestChecker — test coverage (v8/istanbul/clover)
├── clippy.rs        # ClippyChecker — Rust lint warnings
├── metrics.rs       # MetricValue enum, build_unified() merges all checker metrics
├── history.rs       # qualifier.json load/save, Run struct
├── chart.rs         # quality.html generation from template
├── compat_chart.rs  # fallow-chart.html compat update
├── compat_md.rs     # fallow-progress.md compat update
└── summary.rs       # Console summary table
```

### Checker Pattern
Each checker implements `Checker` trait:
```rust
fn can_run(&self, project_dir: &Path) -> bool;  // Does the project have the tool?
fn run(&self, project_dir: &Path, verbose: bool) -> Result<CheckerOutput>;
```

Checkers call `exec::run_command(cmd, args)` — **never** invoke `Command` directly.

### Execution Flow (exec.rs)
1. Check for `docker-compose.yml` in project
2. Find running service (`app`, `web`, etc.)
3. Check if binary exists in container's `node_modules/.bin/`
4. If yes → `docker compose exec -T <svc> node_modules/.bin/<cmd>`
5. If no → host `node_modules/.bin/<cmd>` or `npx <cmd>`

### Output Directory (main.rs)
- `--output-dir` flag overrides everything
- Auto-detect: Next.js (`next.config.*` or `next` in deps) → `public/`
- Auto-detect: Vite (`vite.config.*`) → `public/`
- Fallback: project root
- `qualifier.json` always goes to project root (data, not served)

### Coverage Parsing (vitest.rs)
Fallback chain (first match wins):
1. `coverage/coverage-summary.json` — Istanbul provider
2. `coverage/clover.xml` — v8 provider (parse `<metrics>` tag attributes)
3. `coverage/coverage-final.json` — Istanbul fallback (parse hit counts)

## Build & Run

```bash
# Build release binary
cargo build --release
# Binary: target/release/qualifier

# Run against a project
qualifier --project-dir E:/projects/PropertyShop --note "some note"

# Run in current directory
qualifier --note "some note"

# Override output location
qualifier --output-dir ./reports --note "some note"

# See checker output
qualifier --verbose --note "some note"

# Dry run (no files written)
qualifier --dry-run --note "some note"
```

## Key Decisions & Rationale

| Decision | Why |
|----------|-----|
| Docker auto-detection | Tests need database access; running on host fails silently |
| Binary check in container | fallow isn't installed in container; vitest is — don't assume all tools are there |
| Clover XML parsing | v8 provider (default in vitest) doesn't produce coverage-summary.json |
| Istanbul parser fix | Original code double-counted statements (statementMap + s hits) |
| Output dir auto-detect | Next.js serves `public/` automatically; no manual copy needed |
| qualifier.json in root | Data file, not served; keeps history separate from static assets |
| No XML library | Clover parsing is simple attribute extraction; no dependency needed |

## Testing
- No unit tests yet (TDD debt)
- Integration tested against PropertyShop (Next.js + Docker + PostgreSQL)
- Run `cargo build --release` to verify compilation

## Dependencies
- `clap` — CLI argument parsing
- `serde`/`serde_json` — JSON serialization
- `chrono` — timestamps
- `anyhow` — error handling
- `glob` — file pattern matching
- `regex` — text parsing

## Common Tasks

### Add a new checker
1. Create `src/<name>.rs` implementing `Checker` trait
2. Call `exec::run_command()` for any subprocess
3. Register in `checker::get_checker()` match arm
4. Add detection in `detect::detect_checkers()`

### Modify coverage parsing
- Edit `vitest.rs` — coverage parsing is self-contained
- Clover XML: parse `<metrics>` tag attributes (statements, coveredstatements, etc.)
- Istanbul: parse `coverage-final.json` hit counts

### Change output behavior
- Edit `main.rs` `resolve_output_dir()` function
- Add new framework detection (e.g., Remix, SvelteKit)
