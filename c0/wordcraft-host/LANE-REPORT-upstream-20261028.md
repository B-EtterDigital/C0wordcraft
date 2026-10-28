# Lane report: upstream pull 20261028 (storytold/wordcraft d522bfd8b191 into c0/main)

Status: **proofs incomplete**. The merge and fixes are committed on `fix`. The ui-egui rerun after
the last fix, the wasm check and clippy were queued at the smoac gate behind other lanes' landings
and had not run when this report was written. The last result that ran is recorded below.

Base: c0/main `9c0224862fdc`. Upstream: `d522bfd8b191`. Commits on `fix`:

| sha | what |
|---|---|
| `ebed335c` | merge: upstream d522bfd8b191 into c0/main (conflict resolution, see below) |
| `ee0eb213` | fix(c0): list the host switches in window_ui's UiState pattern (compile fix) |
| `cf3f4a91` | chore(c0): cargo re-locks ui-egui's base64 to 0.23.1 (cargo's output, not a hand edit) |
| `73473455` | fix(c0): keep the host's Save Changes prompt for file.open and file.new; Id::unique; PATCH-DEBT |

No "keep upstream trademarks and CI out of c0/main" commit was made: after the merge, the only
tracked paths under `docs/brand/` and `.github/workflows/` were `docs/brand/LICENSE-brand.txt`
(kept by the brief's exception) and nothing under `.github/workflows/` (already deleted in the
merge, see below). `.github/dependabot.yml` (upstream) is still tracked; the brief's cleanup
names only the workflows folder, so it was left in place.

## Conflicts and how each was resolved

Six modify/delete conflicts, all `.github/workflows/*` (ci, freebsd, packaging-lint, printing,
release, windows-arm64). Our side had deleted them; resolved as deletions, never resurrected.

Five content conflicts. Upstream restructured the UI crate heavily (control, dialogs, lib, ribbon
split into modules), so each of our hunks was re-applied at its new location, taken from upstream's
file. Nothing was dropped.

| file (c0/main) | resolution | new location |
|---|---|---|
| `apps/wordcraft-web/src/web.rs` | Both kept: upstream's `?host=parent` author block (needs `&mut app`), then our `attach`. Re-applied in place. | same file |
| `crates/ui-egui/src/control.rs` | Upstream's file took over; our `c0.*` arm moved into the new `control/handler.rs` match, first arm. | `control/handler.rs` |
| `crates/ui-egui/src/dialogs.rs` | Moved out: the About block now lives in `dialogs/dispatch.rs`; our delegation applied there. | `dialogs/dispatch.rs` |
| `crates/ui-egui/src/lib.rs` | Moved out: `ui_command` seam to `app/commands.rs`; the two `works_without_document` names to `app/commands.rs`; the side-pane call to `app/frame.rs`. Module decl and UiState fields stayed in `lib.rs`. | `lib.rs`, `app/commands.rs`, `app/frame.rs` |
| `crates/ui-egui/src/ribbon.rs` | Moved out: Voice group to `ribbon/home.rs`; Help community button to `ribbon/review.rs`. Zotero tab filter and the float wrapper stayed in `ribbon.rs`. | `ribbon.rs`, `ribbon/home.rs`, `ribbon/review.rs` |

Re-applied rather than moved: the web-build guards (Zotero tab, Dictate group) and the float
wrapper. No upstream switch exists for them; moving them would need a new upstream seam. Listed as
re-applied in PATCH-DEBT.

Dropped with proof: none.

Auto-merged, read in final position: `backstage.rs`, `chrome.rs`, `ribbon_fit.rs`, `theme.rs`,
`apps/wordcraft-web/Cargo.toml`, `apps/wordcraft-web/src/main.rs`, `crates/ui-egui/Cargo.toml`,
`Cargo.lock`. All sit in the function they were written for. `ribbon_fit.rs` now calls the c0
helper `levels`, which reproduces upstream's `all(...)` guard exactly at the default (not compact).

## Two fixes after the merge

1. `window_ui.rs`: upstream destructures `UiState` exhaustively, so the two new switch fields had
   to be named there (`title_bar: _`, `community_links: _`). This is the forced debt below.
2. `c0_files.rs`: upstream no longer prompts before `file.open` or `file.new` when the document
   has unsaved work. Upstream's `discards_document` is now only `file.close`; opening opens a file
   in its own window. Our host contract still expects Save / Don't Save / Cancel. `save_changes_first`
   raises the same `SaveChanges` dialog that `run()` used to raise, and replays the command on
   Save or Don't Save. The test `open_and_new_preserve_unsaved_work_until_normal_prompt_is_answered`
   failed first at `c0.openFile`, then at `c0.newDocument`; both are fixed by this commit. **The
   rerun that proves it is still queued.**

## PATCH-DEBT before and after

- Before (c0/main, measured against ffa51443): 54 added lines in 13 upstream files.
- After (measured against d522bfd8b191, the new upstream): **56 added lines in 18 upstream files**.
- The +2 is `window_ui.rs`, forced by upstream's exhaustive `UiState` destructure. Every other
  line moved with its upstream code and kept its count.
- Several lines moved files (the 13 files became 18 because upstream split files): see the table in
  `c0/wordcraft-host/PATCH-DEBT.md`, which now names the new locations and records the coupling
  rows for the Save Changes prompt and the folded right dock.

## Tests

Recorded from the logs in `/srv/smoac/build/sow-fix-wordcraft-20261028-logs/`:

| binary | result | when |
|---|---|---|
| wordcraft-ui-egui lib | 989 passed, 0 failed, 2 ignored | run 2 (before the prompt fix; the prompt is not in lib tests) |
| c0_chrome | 3 passed | run 2 |
| c0_control_passes | 3 passed | run 2 |
| c0_host_contract | 19 passed, **1 failed** (`open_and_new_preserve_unsaved_work_until_normal_prompt_is_answered`) | run 2; failure fixed in `73473455`, **rerun queued** |
| c0_toolmodes | 11 passed | run 2 |
| it (integration) | 10 passed | run 2 |
| wordcraft-web (`--features page-control`) | 11 passed | run 1 |

Upstream-red: none observed. The only failing test is ours and is fixed in `73473455`. No
pure-upstream worktree run was needed.

Not yet run: the ui-egui rerun after `73473455`, `cargo check --target wasm32-unknown-unknown -p
wordcraft-web --features page-control`, the wasm page build and its size, and clippy. The wasm
check and the clippy run are the remaining proofs. The page build was not attempted.

Native `cargo check -p wordcraft-ui-egui -p wordcraft-web --features wordcraft-web/page-control`
passed on `cf3f4a91`. The four `Id::new` deprecation warnings were ours; `73473455` replaces them
with `Id::unique` (`egui::Id::new` is `Id::unique(source)` in this egui).

Host load: before the first test run 17.3; after it 18.5. Disk: 83% at the start, 84% after the
first run, 79% when the second run finished.

## New upstream commands and menus

Heuristic: string-literal ids present in upstream and absent from c0/main (grep over `crates/` and
`apps/`; may miss ids built at runtime).

- `design.scaleFontSizes`, `equation.deleteSurrounding`, `file.closeDocument`, `format.b`,
  `help.checkForUpdates`, `ui.checkForUpdates`, `ui.paneDock`, `view.readLayout`,
  `view.readPageColor`, `view.stylesPaneDock`, `view.switchWindows`.
- Menu: Help group gains "Check for Updates…" (`ribbon/review.rs`).
- Menu: Home > Voice group, Dictate, is hidden on the web build by our guard (unchanged behaviour
  on desktop).

## What upstream added, in plain words

513 non-merge commits; 20 carry a feat, perf or fix prefix. Items relevant to the host:

- feat: startup update prompt and self-update (`craft-update`), with a check command in Help.
- feat: installer defaults to a per-user ArtCraft folder; installer scope and architecture fixes.
- feat: task panes can be docked left, right or floating; placement is saved in `ui.json`.
- feat: view.switchWindows and multi-window handling: every way of opening a file goes through one
  rule (an untouched blank document is replaced; otherwise the file opens in its own window).
- feat: web host bridge save-as handling (#3822): a save under an unknown name is Save As.
- fix (save/undo): undo steps a command that changes nothing no longer leave; a replaced document
  starts a new undo history (#4001); commands that change nothing keep caches (#4004).
- fix (save): private save keeps the glossary's parts; .doc drawings budgeted (#3692);
  DOCX style replacement across saves (#2469).
- fix (crash): a tiny sharpen value no longer panics in the blur (#3646).
- fix (updates, installer): update status messages localised; self-update result shown in the
  prompt.
- Many format and layout fixes (RTF, ODT, markdown import, table contact); not listed further.

## Clashes with our UI

- **Folded right dock (`c0_chrome.rs`, `RIGHT`):** upstream now docks each task pane left, right
  or floating. The strip knows six right-edge flags only. A pane docked elsewhere is still hidden
  by the fold, and Navigation, Clipboard, FormatShape and SmartArt Text are not folded at all.
  Not tested; recorded as a coupling in PATCH-DEBT.
- **Two embedding bridges:** upstream's `?host=parent` (`apps/wordcraft-web/src/host.rs`) and our
  `page_control` coexist. Both attach to the runner; they do not conflict in code, but they are two
  paths for a parent page to drive the app.
- **Opening files:** upstream opens a file beside unsaved work. Our host contract replaces, so the
  prompt is raised in c0 code. A document kept open in another window after Don't Save is a
  consequence we did not test.
- **Update prompt and self-update:** upstream's startup prompt and self-update may reach the
  network from an embedded host. Not checked in the web build.
- **Home, menus, theme, community links, agent server:** the Help group and the Discord footer
  entries are guarded as before; the new Check for Updates button in Help is not guarded.
  Upstream's theme and palette code is unchanged by us except the 4 lines in `theme.rs`; the
  agent server (`c0.*` dispatch) moved into `control/handler.rs` with no behaviour change.

## Assumptions I could not confirm

- That the 56-line count is the number the controller expects. I measured against d522bfd8b191
  the way PATCH-DEBT defines the column.
- That `save_changes_first` matches upstream's `run()` prompt in every case: `run()` also flushes
  comment drafts and refuses on a refused flush. For file.open and file.new the c0 path does not
  flush comment drafts first. The test does not cover this.
- That the upstream `file.new` and `file.open` replay (on Save or Don't Save) behaves like the old
  `run()` replay. The test passes only on the prompt side until the queued rerun reports.
- That the new upstream command list is complete; it comes from a heuristic grep.
- The wasm page build (`trunk build --release`) was not run. The size cap (46137344 bytes) cannot be
  checked until it is.

LANE-BLOCKED wordcraft `fix` at 73473455: proofs queued at the smoac gate behind other lanes (ui-egui rerun after the prompt fix, wasm check, clippy); code and report committed, verification pending.
