# Session — Qualifier Docker Support + Dashboard Overhaul

**Date:** 2026-07-13
**Goal:** Make qualifier work automatically in Docker projects and build a polished dashboard

---

## Context

Qualifier was running vitest on the host where PostgreSQL isn't available → tests fail → coverage is null. Dashboard had no traffic lights, no score, and output files needed manual copying.

## Changes Made

### 1. Docker-Aware Execution

| File | Change |
|------|--------|
| `src/docker.rs` | **New** — detect docker-compose, find running service, check binary exists in container |
| `src/exec.rs` | **New** — unified command runner (Docker first, host fallback) |
| `src/vitest.rs` | Use `exec::run_command()` instead of `run_npx()` |
| `src/fallow.rs` | Use `exec::run_command()` instead of `run_npx()` |
| `src/main.rs` | Added `mod docker; mod exec;` |

**How it works:**
1. Check for `docker-compose.yml` in project
2. Find running service (`app`, `web`, etc.)
3. Check if binary exists in container's `node_modules/.bin/`
4. If yes → `docker compose exec -T <svc> node_modules/.bin/<cmd>`
5. If no → falls back to host (npx)

### 2. Clover XML Coverage Parsing

| File | Change |
|------|--------|
| `src/vitest.rs` | Added Clover XML parser for v8 provider support |

**Coverage fallback chain:** `coverage-summary.json` → `clover.xml` → `coverage-final.json`

### 3. Auto Output Directory

| File | Change |
|------|--------|
| `src/main.rs` | Added `resolve_output_dir()` — detects Next.js/Vite → writes to `public/` |

- `--output-dir` flag overrides auto-detection
- `qualifier.json` always goes to project root (data, not served)
- HTML files go to `public/` for Next.js/Vite (auto-served)

### 4. Dashboard Overhaul

| File | Change |
|------|--------|
| `templates/quality.html` | Complete redesign |

**Changes:**
- **Overall quality score** next to title (green/amber/red)
  - Green = 100%, Amber = 50%, Red = 0% per metric, averaged
- **Traffic light line colors** on charts (green = on target, amber = close, red = bad)
- **Removed dots** from charts — clean line graphs
- **Charts reordered** — higher-is-better on left, lower-is-better on right
- **Hover tooltips** — plain English explanations for each metric
- **Run history table** — sorted latest-first, added Files column
- **Test Files card** — shows actual count (was showing "—")

### 5. History Restoration

| File | Change |
|------|--------|
| `PropertyShop/qualifier.json` | Restored 26 runs of real data from `fallow-chart.html` |

Converted old HTML data rows to qualifier.json schema. Replaced 9 test runs created during development.

### 6. Documentation

| File | Change |
|------|--------|
| `CLAUDE.md` | **New** — project guide for future sessions |

## Commits

| Hash | Message |
|------|---------|
| `ee9dc99` | fix: defer chart drawing to requestAnimationFrame |
| `d8ad000` | feat: Docker-aware checker execution with Clover XML coverage parsing |
| `5c113e1` | feat: auto-detect output directory for Next.js and Vite projects |
| `e10dc4f` | feat: replace timeline cards with run history data table |
| `4aa89ca` | fix: show test_files count in summary card and run table |
| `5a6d86e` | feat: traffic light colors on summary cards |
| `b940d8a` | feat: dashboard overhaul — score, traffic lights, tooltips, layout |

## Results

**Before:**
- Vitest ran on host → PostgreSQL not available → coverage null
- Manual copy of output files to `public/`
- Dashboard: no score, no traffic lights, no tooltips
- 9 test runs in history

**After:**
- Vitest runs in Docker container → PostgreSQL available → real coverage data
- Output files auto-written to `public/` for Next.js projects
- Dashboard: 71% score, traffic lights, tooltips, clean charts
- 28 runs of real history showing full trajectory

## Current Metrics (PropertyShop)

| Metric | Value | Status |
|--------|-------|--------|
| MI | 93.6 | 🟢 |
| Dead % | 0% | 🟢 |
| Dup % | 7% | 🟢 |
| Stmt % | 77.8% | 🟡 |
| Branch % | 66.2% | 🟡 |
| Func % | 87.1% | 🟢 |
| Max CRAP | 992 | 🔴 |
| **Score** | **71%** | **🟡** |

---

## Dashboard Layout Overhaul (Jul 13 ~14:00)

**Trigger:** User wanted score card at top, metric values above their charts, cleaner presentation.

### Commits

| Commit | Message |
|--------|---------|
| `1660f99` | feat: dashboard layout overhaul — score at top, metric values above charts, clean titles |
| `077aecc` | style: brighter chart axis labels and grid lines |
| `237e6bc` | fix: chart timeline labels — clean HH:MM format, proper spacing |

### Changes

**Layout:**
- Overall Quality Score → hero card at top of page (was inline with title)
- Removed separate summary cards row
- Each chart card now has: centered title → metric value (2rem, traffic-light colored) → chart canvas
- Chart titles cleaned (removed target annotations — dashed lines convey that info)
- Chart titles horizontally centered

**Chart Improvements:**
- Timeline labels: up to 6 evenly-spaced `DD HH:MM` labels, rotated 45° (was full ISO string, overlapping)
- Axis labels: `#484f58` → `#8b949e` (brighter, more visible)
- Grid lines: `#21262d` → `#30363d` (more visible)
- Bottom padding: 30 → 40px (room for rotated labels)

**Remote:**
- Added `origin` → `https://github.com/clintmarshall/COMMONTOOLS.git`
- Pushed `main` branch

## Open Questions

- Score button on site UI: embed badge vs API endpoint? (discussed, no decision)
