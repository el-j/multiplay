---
id: PLAN-023
title: Boss-File Refactoring & Modular Architectural Decomposition
status: in_progress
role: architecture
tasks:
  - TASK-070
  - TASK-071
  - TASK-072
---

# PLAN-023: Boss-File Refactoring & Modular Architectural Decomposition

## Objective
Eliminate all oversized, monolithic "boss files" identified in `docs/orchestration-audit.md` section 5. Decompose them into small, human-readable, single-responsibility modules strictly under 300 lines each.

## Architecture Context
- **Audit Grounding (`docs/orchestration-audit.md`)**:
  - Section 5: The repository contains several large single files (> 350-470 lines) that are hard to maintain, hide design drift, and violate modular design principles.
- **Decomposition Targets**:
  - `crates/amp-app/src/main.rs` (477 lines) -> split into CLI, runner, terminal, composition.
  - `crates/amp-adapter-gui/src/app.rs` (447 lines) -> split into view state, panels, event dispatcher.
  - `crates/amp-adapter-gui/src/docking.rs` (349 lines) -> split into geometry, snapping, layout storage.
  - `crates/amp-adapter-cpal/src/lib.rs` (407 lines) -> split into device, stream, ring buffer.
  - `crates/amp-adapter-decoder/src/lib.rs` (388 lines) -> split into demuxer, decoder core, sample converter, seek.

## Tasks
1. [TASK-070](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-070-decompose-app-main-monolith.md): Decompose amp-app/src/main.rs Monolith into Modular Subsystems
2. [TASK-071](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-071-decompose-gui-app-and-docking.md): Decompose GUI App and Docking Modules into Focused Subsystems
3. [TASK-072](file:///Users/rex-fab-alt/Documents/private/multiplay/.claude/tasks/TASK-072-decompose-cpal-and-decoder-adapters.md): Decompose CPAL Sink and Symphonia Decoder into Modular Units

## Definition of Done
- No source file in the entire workspace exceeds 300 lines.
- Each module has a single, well-defined responsibility with dedicated unit tests.
- Zero behavior regressions across CLI, TUI, GUI, and audio engines.
