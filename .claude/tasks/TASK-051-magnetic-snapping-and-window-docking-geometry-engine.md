---
id: TASK-051
title: Magnetic Snapping & Window Docking Geometry Engine
planId: PLAN-016
status: completed
role: frontend_gui
verificationCommand: cargo test -p amp-adapter-gui test_docking
---

# TASK-051: Magnetic Snapping & Window Docking Geometry Engine

## Description
Develop the pure geometric calculation engine for magnetic window snapping and multi-window docking:
1. `DockRect` and `DockEdge`: Represent window boundaries and edge alignments (Left, Right, Top, Bottom).
2. Proximity Detection: Given a moving window rectangle and a set of stationary window rectangles, detect if any edges are within the magnetic snapping threshold (e.g. 14 pixels).
3. Snapping Alignment: Snap the moving window's coordinates flush against the adjacent target window edge.
4. Docked Group Tracking: Maintain a graph of docked windows (`DockGroup`). Dragging the root/anchor window calculates displacement delta and translates all attached children in unison.
5. Tear-Off Mechanism: If a child window is dragged with a displacement exceeding the detach threshold (e.g. 24 pixels), sever the dock relationship, allowing independent movement.

## Verification
```bash
cargo test -p amp-adapter-gui test_docking
```
