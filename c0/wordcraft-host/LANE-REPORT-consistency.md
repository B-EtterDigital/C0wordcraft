# Lane report: SOW consistency (tool modes, right dock, PDF bytes)

## Tool-mode decision

WordCraft has a ribbon, not a tool palette. The ribbon is never removed; the PhotoCraft keys map onto it
using what the app already does (each ribbon group already has FULL / COMPACT / COLLAPSED fit levels):

| Key | Meaning here |
|---|---|
| `toolbar` (default true) | ribbon body shown; false = collapsed to its tabs (inverse of `ribbon_collapsed`) |
| `toolbar_double` | **compact single row**: every group at its compact level (small buttons lose labels, shorter Styles gallery); groups fold to drop-downs only if even that is too wide |
| `toolbar_floating` | the ribbon body in a movable strip over the page (drag the grip); WordCraft has no floating palette of its own (the mini toolbar is selection-bound), so this is the "movable strip within the canvas area"; tabs stay docked |

`toolbar_double` + `toolbar_floating` combine. "Two columns" has no meaning for a horizontal ribbon, so
`toolbar_double` is the density mode; this is a judgment call. Compact does not reduce the ribbon's height
(small buttons already stack three high); it narrows it.
`right_dock_collapsed`: Styles, Style Inspector, Accessibility, Alt Text, Restrict Editing and Comments fold
to a 34-point icon strip; clicking an icon (or opening any pane by command) opens it and unfolds. Immediate,
no animation. `c0.inspect.panes` reports `toolbar`, `toolbar_double`, `toolbar_floating`, `right_dock_collapsed`.
Nothing is saved to prefs; defaults equal upstream.

## Extra

`c0.documentBytes {format:"pdf"}` returns the engine's PDF writer output (`%PDF-` verified; read-only, no
revision/dirty change). Other formats are still refused.

## Commits (on c0/main)

See `git log`: `feat(c0doc)` for the modes, dock and PDF, `test(c0doc)` for `tests/c0_toolmodes.rs`, `docs(c0doc)` for this report, README and PATCH-DEBT. An earlier `wip` commit holds the unbuilt first draft.

## Tests (all under `smoac-slot run heavy 2`, CARGO_BUILD_JOBS=2)

- `cargo test -p wordcraft-ui-egui`: 720 passed, 2 ignored (lib); integration: c0_chrome 3, c0_control_passes 3,
  c0_host_contract 15 (PDF assertion updated), c0_toolmodes 11 (new), native_print 8, breaks_menu 2.
- `cargo test -p wordcraft-web --features page-control`: 10 passed.
- wasm check (`--target wasm32-unknown-unknown -p wordcraft-web --features page-control`): pass; 3 warnings are
  upstream dead code in the engine, not ours.
- clippy `-D warnings` over ui-egui and web with all targets: clean.
- Headless Chromium smoke (`c0/wordcraft-host/smoke.mjs` against the built dist): 19/19. Sets full, double,
  collapsed, floating, floating+double and dock-folded, reads each back, screenshots (floating and folded
  checked by eye), unknown/invalid reporting, PDF bytes, ui.render, no JS exceptions.

## Page build and wasm size

`trunk build --release` (c0/wordcraft-host): pass, 11m02s. `wordcraft-web-bee0dab2e683c20a_bg.wasm` =
44,154,526 bytes (previous lane's pin build: 44,322,356 bytes; this one is smaller, a different toolchain
state/upstream-free build). Pin cap value: see assumptions.
Load before (`/proc/loadavg`): 17.06 16.89 15.39. After: 6.75 9.14 12.45. Shared host, not a benchmark.

## Patch debt

Ratchet 50 -> 54 (lib.rs +1, ribbon.rs +2, ribbon_fit.rs +1, all replaced/added single lines), under 70.

## Assumptions I could not confirm

- The "pin cap" for wasm size is not stated anywhere in the repo or the dispatch; I report the size and the delta
  to the last build only.
- That compact (labels dropped, same height) satisfies "compact single row"; a shorter ribbon would need a new
  layout of upstream's 3,400-line ribbon.
- On the first frame after switching to a tab the ribbon draws FULL once before the compact plan applies
  (upstream's fit plan needs the tab's group order); one frame, not visible in the smoke.
- Floating strip position is clamped to the area below the tab strip, which includes the ruler row; it can
  cover the ruler and top of the page by design.
- Dock-fold hides the right panes by clearing their view flags around `panes::show` and restoring them in the
  same call; a future pane added upstream on the right needs a `RIGHT` entry.
- Pushing was not done; the first clone over ssh failed, so I cloned over https (read-only).
