# RFC 0011: Defer split-screen until after PC 1.0

- **Number:** 0011
- **Authors:** @lxstyo/agent
- **Status:** in review
- **Approvers:** owners of `ss2-render`, `ss2-platform`, `ss2-ui`, and `ss2-net`, plus one other maintainer
- **Review SLA:** 3 working days after "in review"
- **Roadmap task:** T04-034

## Summary

Keep one gameplay viewport and one local player per PC client through M8. Reach the co-op
milestones with separate clients connected to the same authoritative server. Reassess
two-player split-screen after PC 1.0 with a prototype and a separate implementation RFC.

## Motivation

M4 requires two PC clients to walk together. M6 requires the first level to be playable in
co-op. Both milestones exercise server authority, prediction, and reconciliation per client.
They can ship with a single camera and one set of local controls in each client. Split-screen
would add a distinct set of input, rendering, and performance requirements before that baseline
is available. [Milestones](../roadmap/README.md#5-milestones-and-exit-criteria)

The current renderer contract has one camera and one UI draw list per submitted `RenderWorld`.
It interpolates by simulation tick, and resources are identified within a renderer instance.
The app currently sends one menu draw list per redraw. Platform events have gamepad IDs,
while action bindings use any connected pad and the app feeds one menu input stream. Two
viewports alone would leave both players sharing input and UI state. [Renderer contract](0002-render-frontend-api.md) ·
[Platform contract](0003-platform-api.md) · [UI framework](0010-ui-framework.md)

## Detailed design

### PC release

Measure one viewport per client as part of the normal co-op and rendering acceptance runs.
Keep headless servers free of renderer dependencies. Exercise independent input and cameras in
M4's co-op walk. Validate per-client HUDs in the M6 vertical slice.

### Scope of a later prototype

Start with two local players in one desktop window and two gamepads. Connect both players to
one authoritative session. Compare two ordinary client connections with a single connection
carrying two local player identities. Each player needs separate input commands, prediction,
reconciliation, and join or disconnect handling under either transport design.

Extract the shared world once per tick. Each player needs a camera, independently culled scene,
viewport aspect ratio, and clipped HUD with a per-viewport safe area. Measure the cost of camera-
dependent passes separately from reusable scene data and GPU resources. Use a window-level menu
for shared actions and route focus changes to the local player who initiated them.

Compare CPU and GPU frame time with the one-viewport baseline in the same PC co-op scene. Keep
player count and movement fixed across runs. Report window size, split orientation, render
settings, and peak process and GPU memory. Verify that resizing, disconnecting a controller,
and dropping the second player preserve the first player's controls. Clear held inputs when the
window loses focus and verify that focus regain restores the right assignments. The renderer and
platform owners should bring those measurements and a HUD legibility check to the implementation
RFC. [UI scaling](../UI_SCALING.md) ·
[Performance budgets](../PERF_BUDGETS.md)

## Alternatives considered

- **Build split-screen for M4.** A single machine could demonstrate local co-op early, with new
  work on per-device bindings, multiple participating clients, and per-view drawing on the M4
  critical path. M4's two-client test already covers the priority network behavior.
- **Generalize the renderer to accept a list of views now.** This would let renderer work move
  independently, but the first use would still need local player identities, UI focus, and
  measurements. A view contract informed by that prototype can account for shared and per-view
  passes together.
- **Open two windows in one process.** Separate surfaces simplify some viewport decisions, but
  the app would need to manage focus, placement, and device routing across windows. Separate
  PC clients are sufficient for the current milestones.

## Compatibility and migration

If the prototype moves forward, propose compatible single-view defaults and a migration for
saved bindings. Prototype local player ownership in the session before exposing per-view
rendering and HUD state through a public contract.

## Open questions

- The netcode owner should choose the connection model after measuring the two-client prototype.
- The platform and UI owners should decide how a second player signs in and whether to support
  a keyboard and a gamepad in addition to two gamepads.
- The audio owner should choose how to mix two listener positions into one speaker output.
