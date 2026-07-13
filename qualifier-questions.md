# Qualifier — Open Questions

> Brainstormed 2026-07-13. Link: [[qualifier.md]]

## 1. `qualifier init` — Bootstrapping a New Project

- **Discovery:** Sniff `package.json` / `Cargo.toml` / `pyproject.toml` to infer ecosystem? Or explicit `--lang=ts`?
- **Tool installation:** Does `init` actually `npm install --save-dev ...` or just write config files?
- **Opinionated presets:** `--preset=minimal` (fallow + tests) vs `--preset=full` (+ lighthouse + coverage + pre-commit)?
- **Config generation:** Does `init` create `vitest.config.*`, etc., or assume they exist?
- **Baseline run:** Does `init` run a full qualifier pass to seed `qualifier.json`?

## 2. Checker Plugin Architecture

- **Binary coupling:** All checkers compiled in vs dynamic plugins vs config-driven external checkers?
- **Concurrency:** `Checker` trait is `Send + Sync` — parallel execution? Contention on disk/ports?
- **Error isolation:** One checker crashes → partial result or total fail? Error strategy?

## 3. Tool Insertion Mechanics

- **Version pinning:** Track "recommended versions"? Bump on `init`? Pin to major?
- **Platform parity:** Lighthouse needs a running dev server — how to handle checkers needing runtime state?
- **Idempotency:** `init` twice — re-add or skip?
- **Existing projects:** Audit what's there and fill gaps vs greenfield only?

## 4. Data Model & History

- **Schema evolution:** `version: 1` — migration strategy for old `qualifier.json`?
- **History retention:** Runs append forever. Cap? Prune? Archive?
- **Git noise:** AI-triggered runs = lots of commits. Frequency control? Squashing?

## 5. CI & Gates

- **Thresholds:** Per-metric or overall grade with defaults? Who sets them?
- **Regression definition:** Any metric wrong way, or only beyond threshold?
- **GitHub Action:** Just `qualifier --ci` in a workflow or separate runner?

## 6. HTML Dashboard

- **Charts:** Hand-rolled canvas or inline charting library?
- **Serving:** `fetch()` blocks on `file://`. Requires HTTP server. Does `init` set up copy/symlink?
- **Mobile:** Responsive or desktop-only?

## 7. MCP / AI Integration

- **MCP server:** Qualifier as MCP server or wrapper?
- **Auto-commentary grounding:** AI claims "MI improved due to Header" — how does it know which file?
- **Pre-commit hook:** `init` installs `husky` / `git hooks/pre-commit`?

## 8. Cross-Platform

- **Windows binary:** Rust compiles natively. Checkers like lighthouse/npx differ on Windows.
- **Path handling in JSON:** Forward slashes for portability?

## Suggested `qualifier init` Priority

1. Ecosystem detection (sniff project files)
2. Preset selection (`minimal` / `full` / custom)
3. Write config files
4. Install tools
5. Drop `quality.html`
6. Seed `qualifier.json` with baseline run
7. Optional: install pre-commit hook
