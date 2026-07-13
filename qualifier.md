# Qualifier — Universal Project Quality Dashboard

## What

A Rust CLI that auto-detects your project's tooling, runs quality checks, and writes the results to `qualifier.json`. A companion HTML page reads that JSON and renders a complete quality dashboard.

One binary. One JSON file. One HTML page. Zero config.

```sh
qualifier              # detect, run, write qualifier.json
qualifier --note "Phase 2 done"   # with commentary
```

## Architecture

```
┌─────────────────────────────┐
│  qualifier (Rust binary)    │
│                             │
│  1. Scan project            │
│  2. Run available checkers  │
│  3. Merge with history      │
│  4. Write qualifier.json    │
└──────────────┬──────────────┘
               │
               ▼
┌─────────────────────────────┐
│  qualifier.json             │
│                             │
│  {                          │
│    "runs": [                │
│      {                      │
│        "ts": "...",         │
│        "note": "...",       │
│        "metrics": { ... },  │
│        "commentary": "..."  │
│      }                      │
│    ]                        │
│  }                          │
└──────────────┬──────────────┘
               │
               ▼
┌─────────────────────────────┐
│  quality.html               │
│                             │
│  Reads qualifier.json       │
│  Renders:                   │
│  - Summary cards            │
│  - Trajectory charts        │
│  - Delta table              │
│  - Commentary timeline      │
│  - Raw metrics (collapsible)│
└─────────────────────────────┘
```

## The JSON Schema

```jsonc
{
  "version": 1,
  "project": "PropertyShop",           // auto-detected from package.json/Cargo.toml
  "runs": [
    {
      "ts": "2026-07-12T10:00:00Z",
      "note": "Phase 1 complete",      // --note flag
      "commentary": "",                // optional rich commentary (added via --comment or editor)
      "checkers": {                    // which checkers ran
        "fallow": { "version": "3.3.0", "elapsed_ms": 4065 },
        "vitest": { "version": "3.0.0", "elapsed_ms": 78970 }
      },
      "metrics": {
        "loc": 22919,
        "dead_file_pct": 6.8,
        "dead_file_count": 9,
        "dead_export_pct": 0,
        "avg_cyclomatic": 2.7,
        "p90_cyclomatic": 5,
        "mi": 91.6,
        "max_crap_app": 1260,
        "dup_lines": 3719,
        "dup_pct": 18.4,
        "clone_groups": 22,
        "cov_stmt": 49.93,
        "cov_branch": 48.78,
        "cov_func": 51.3,
        "cov_lines": 50.21,
        "test_count": 522,
        "test_files": 48
      }
    }
  ]
}
```

**Commentary** replaces the MD changelog. Each run can carry:
- `note` — one-liner (from `--note`)
- `commentary` — free-form text explaining what changed and why metrics moved

The HTML dashboard renders commentary as a timeline alongside the charts.

## Auto-Detection

Qualifier scans the project root and runs whatever it finds:

| Signal | Checker | Command |
|--------|---------|---------|
| `package.json` + `fallow` | fallow | `npx fallow --format json` |
| `vitest.config.*` | vitest | `npx vitest run --coverage` |
| `jest.config.*` | jest | `npx jest --coverage --json` |
| `next.config.*` | lighthouse | `lighthouse http://localhost:3000 --output=json` |
| `Cargo.toml` | clippy | `cargo clippy --message-format json` |
| `pyproject.toml` + pytest | pytest | `pytest --junitxml=- --cov` |
| `go.mod` | gotest | `go test -coverprofile=cover.out ./...` |

Nothing to configure. If it's there, it runs. If it's not, it skips.

## The HTML Dashboard

A single static file. No build step, no CDN, no framework. Reads `qualifier.json` via `<script src="qualifier.json">` or `fetch()`.

### Sections

1. **Summary cards** — current values with delta vs previous run
   - MI, dead files, duplication, coverage, test count, max CRAP
   - Green/red arrows for direction

2. **Trajectory charts** — one canvas per metric, line chart over time
   - Target lines (MI ≥ 80, coverage ≥ 80%, dead ≤ 5%, dupes ≤ 10%)
   - Data points labelled with values

3. **Commentary timeline** — chronological list of runs with notes and commentary
   - Replaces the MD changelog
   - Collapsible per-run details

4. **Delta table** — current vs previous run, all metrics
   - Color-coded: green for improvement, red for regression

5. **Raw JSON** — collapsible, for tooling/inspection

### Embedding in the project

```
my-project/
├── qualifier.json            ← generated, committed to git
├── quality.html              ← static, committed to git
├── src/app/page.tsx          ← links to /quality.html
└── public/
    ├── qualifier.json        ← symlink or copy
    └── quality.html          ← served at /quality.html
```

## Rust Binary — Core Flow

```rust
fn main() -> Result<(), Box<dyn Error>> {
    // 1. Detect project
    let project = detect_project()?;

    // 2. Run available checkers
    let metrics = run_checkers(&project)?;

    // 3. Load existing history
    let mut history = load_history("qualifier.json")?;

    // 4. Append new run
    history.runs.push(Run {
        ts: Utc::now(),
        note: cli.note,
        commentary: cli.commentary,
        metrics,
        checkers: checker_results,
    });

    // 5. Write back
    save_history(&history, "qualifier.json")?;

    // 6. Display summary
    print_summary(&history.runs.last().unwrap(), history.runs.len() > 1.then(|| history.runs[history.runs.len()-2]));

    Ok(())
}
```

## Checker Trait

```rust
pub trait Checker: Send + Sync {
    fn name(&self) -> &str;
    fn can_run(&self, project: &ProjectInfo) -> bool;
    fn run(&self, project: &ProjectInfo) -> Result<BTreeMap<String, MetricValue>, CheckerError>;
}
```

## CLI

```
qualifier [OPTIONS]

Options:
  --note <TEXT>       One-line note for this run
  --comment <TEXT>    Commentary (rich text, explains what changed)
  --json <PATH>       Path to qualifier.json (default: qualifier.json)
  --html <PATH>       Path to quality.html template (default: bundled)
  --dry-run           Run checks but don't write
  --ci                Exit non-zero on regression
  --threshold <K=V>   CI threshold (e.g., --threshold mi=-2)
  --verbose           Show checker output
```

## Commentary Workflow

Instead of maintaining a separate MD changelog, commentary lives in the JSON:

```sh
# Quick note
qualifier --note "Fixed Header CRAP score"

# Detailed commentary
qualifier --note "Refactored Header" \
  --comment "Split Header into 3 components. CRAP dropped from 1260 to 340. MI improved 0.3 points."

# Edit commentary post-hoc (qualifier supports editing the last run)
qualifier --edit-comment "Also fixed dead exports in contact.repo"
```

The HTML dashboard renders commentary as a proper timeline with timestamps, notes, and full commentary text — replacing the need for `fallow-progress.md` entirely.

## Design Principles

1. **Two files, one job each** — Rust writes JSON, HTML reads JSON. No overlap.
2. **Zero config** — auto-detect tools, run what's available
3. **JSON is the source of truth** — everything flows from `qualifier.json`
4. **Commentary replaces MD** — notes and commentary in JSON, rendered by the dashboard
5. **Committed to git** — both files are part of the repo
6. **No network** — everything local, no APIs, no CDNs
7. **Extensible** — implement `Checker`, it auto-detects via `can_run()`

## AI Integration

Qualifier exposes a clean JSON interface that AI agents can consume and act on.

### Automatic Runs

The AI secretary runs qualifier after meaningful changes:

```
User: "I just refactored the Header component"
AI: → runs `qualifier --note "Refactored Header"`
    → reads qualifier.json
    → reports: "MI improved 0.3, CRAP dropped from 1260 to 340. Coverage stable at 49.9%."
```

No manual invocation needed. The AI watches for code changes and triggers qualifier automatically.

### Auto-Commentary

Qualifier can generate commentary by comparing runs:

```json
{
  "ts": "2026-07-12T10:00:00Z",
  "note": "Refactored Header",
  "commentary": "Split Header.tsx into 3 components. CRAP dropped from 1260 to 340 (↓73%). MI improved 0.2 points. Dead files unchanged at 6.8%. Coverage diluted 0.1% due to new untested components.",
  "ai_generated": true
}
```

The AI reads the delta between runs, understands what changed, and writes a human-readable summary into the JSON.

### MCP Integration

Qualifier exposes tools for AI agents via MCP:

```json
{
  "name": "qualifier_run",
  "description": "Run quality checks and return results",
  "parameters": {
    "note": "string (optional)",
    "checkers": "array (optional, default: auto-detect)"
  }
}

{
  "name": "qualifier_trend",
  "description": "Analyze quality trends over time",
  "parameters": {
    "metric": "string (mi, coverage, dead, dupes, crap)",
    "direction": "string (improving, declining, stable)"
  }
}

{
  "name": "qualifier_fix",
  "description": "Suggest fixes for quality regressions",
  "parameters": {
    "metric": "string",
    "threshold": "number (optional)"
  }
}
```

### Workflow

```
1. AI detects code change (git diff, file save, PR update)
2. AI runs qualifier
3. AI reads qualifier.json delta
4. AI generates commentary explaining the change
5. AI commits qualifier.json with note
6. AI reports to user: "Quality check passed. MI 91.6, coverage 49.9%. No regressions."
```

### Future Vision

- **Pre-commit hook** — qualifier runs automatically, blocks commit on regression
- **PR quality gate** — AI reviews qualifier.json delta, comments on PR
- **Trend prediction** — AI warns: "Duplication trending up 0.4%/week. Will exceed 20% in 6 weeks."
- **Auto-fix suggestions** — AI identifies specific files causing regressions and suggests fixes
- **Client reporting** — AI generates plain-language quality reports for non-technical stakeholders

## Future

- **WASM build** — run checks in the browser for projects without local tooling
- **Badge** — `![quality](https://qualifier.dev/badge?repo=owner/repo)` reads committed `qualifier.json`
- **PR comments** — GitHub Action compares `qualifier.json` delta
- **Custom checkers** — any CLI tool with JSON output is auto-detected
- **Multi-project** — run qualifier across a monorepo, aggregate into one dashboard
