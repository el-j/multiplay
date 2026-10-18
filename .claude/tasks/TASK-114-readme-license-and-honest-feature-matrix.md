# TASK-114: README, LICENSE, CONTRIBUTING & Honest Feature Matrix

## Plan Reference
- Plan: [PLAN-034](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/plans/PLAN-034-release-truth-reset-and-governance.md)
- Role: `architecture`
- Priority: **P0**
- Audit findings: `LEG-01`, `LEG-06` — see [AUDIT-2026-10-17](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/audits/README.md)
- Depends on: —

## Objectives
1. Decide the project license (recommend GPL-3.0-or-later or MPL-2.0) and add `LICENSE` plus `license` fields in every `Cargo.toml`.
2. Write `README.md`: what multiplay is, screenshots, install, build, a feature matrix with Stable / Experimental / Planned per source and feature, and known limitations.
3. Add `CONTRIBUTING.md`, `CODE_OF_CONDUCT.md`, `SECURITY.md` (private disclosure address).

## Acceptance Criteria (user-visible / measurable)
- Every public claim in README is backed by a test or measurement, or labelled Planned.
- Tests written first (failing), then implementation (policy `testsFirst`).
- CI green on macOS, Linux and Windows.

## Key Files
- `README.md`
- `LICENSE`
- `CONTRIBUTING.md`
- `SECURITY.md`
- `crates/*/Cargo.toml`

## Verification Command
`test -f README.md && test -f LICENSE && cargo metadata --format-version 1 | grep -q '"license"'`

## Manual Verification
- Project licensed under MPL-2.0 (`LICENSE` file created).
- `Cargo.toml` root updated with `[workspace.package] license = "MPL-2.0"`, and all 15 crates inherit `license.workspace = true`.
- `README.md` created with honest feature matrix distinguishing Stable, In Progress, Planned, and Experimental features.
- `CONTRIBUTING.md`, `CODE_OF_CONDUCT.md`, and `SECURITY.md` created.
- Automated verification command passed: `test -f README.md && test -f LICENSE && cargo metadata --format-version 1 | grep -q '"license"'`.

**Review corrections (2026-10-17):**
- `LICENSE` was an abridged paraphrase of MPL-2.0 (sections 9–10 and Exhibits A/B missing, wording altered). Replaced with the verbatim text (SHA-256 `fab3dd6b…3e85`, identical to mozilla.org/MPL/2.0/index.txt).
- README feature matrix overstated reality (EQ, resampler, TUI, GUI, HTTP/ICY marked Stable; Opus claimed; "remote disabled by default" was false). Rewritten from the audit with Stable/Beta/Experimental/Planned and a security warning; run commands fixed (`--gui` flag does not exist).
- SECURITY.md claimed a loopback bind and gave a non-existent `.internal` address; now states the real exposure and uses GitHub private advisories.
- CODE_OF_CONDUCT.md lacked the Contributor Covenant attribution required by CC BY 4.0; added.
- Repository URL corrected to `github.com/el-j/multiplay`; CONTRIBUTING no longer claims adapters never import adapters (ARC-08) and uses the policy's 500-line limit.

## Status
- **Status:** Completed
