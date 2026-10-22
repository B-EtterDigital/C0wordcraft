# Patch debt: lines of ours inside upstream files

All substantive host code lives in new files. This table counts every modified upstream file,
including replacement lines, brace/indentation changes and Cargo's dependency bookkeeping.
The `Lines` column is the added column of
`git diff --numstat --diff-filter=M main HEAD -- . ':!c0'` on `c0/main`.
It is not a net line count. New files and everything under `c0/` are excluded.

| File | Lines | What | Why | Offered upstream |
|---|---|---|---|---|
| `Cargo.lock` | 1 | Register the existing workspace base64 dependency for ui-egui | Keep locked builds reproducible | Not yet |
| `apps/wordcraft-web/Cargo.toml` | 3 | Default-off `page-control` feature enables rfd's file-handle-inner accessor; existing serde_json reused | Build the existing binary as an embeddable guest with bounded picker reads | Not yet (generic feature hook) |
| `apps/wordcraft-web/src/main.rs` | 2 | Declare `page_control` for wasm and native tests when the feature is enabled | Keep native feature tests compiled (K-41) | Not yet |
| `apps/wordcraft-web/src/web.rs` | 7 | Name/pass the WebRunner (4 added, 1 replaced); attach the bridge (2); expose the existing picker file-count cap to the page picker (1 replaced) | Answer requests without presentation and preserve the upstream count limit | Not yet (generic attachment seams) |
| `crates/ui-egui/Cargo.toml` | 1 | Existing workspace base64 dependency | Strict file payload validation in the host module | Not yet |
| `crates/ui-egui/src/lib.rs` | 14 | Declare `c0_host` (1); two default-true, non-persisted UiState fields (5), their defaults (2), UI command helper seam (3), allow those commands with no document (2); the side-pane call goes through the host dock wrapper (1 replaced) | Host title/community switches in the existing state and command path | Not yet (generic embeddability switches/seam) |
| `crates/ui-egui/src/control.rs` | 1 | One `c0.*` dispatch arm | All host requests use the real control queue | Not yet (fork namespace) |
| `crates/ui-egui/src/chrome.rs` | 4 | Return when the title row is hidden (3); gate the Discord pill (1 replaced) | The host owns its title and community chrome | Not yet (generic switches) |
| `crates/ui-egui/src/backstage.rs` | 3 | Skip the existing Discord footer entry when community links are hidden | Keep About while suppressing promotion | Not yet (generic switch) |
| `crates/ui-egui/src/ribbon.rs` | 12 | Guard the existing Help > Community Discord button (1 replaced, 2 braces); web build only: drop the Zotero tab (1 replaced) and the Home > Voice group with Dictate (comment, guard, brace, 3 re-indented) | Hide the entry without changing the Help tab; neither the Zotero desktop link nor dictation works in a browser | Not yet (the web-build guards are offerable as is) |
| `crates/ui-egui/src/ribbon_fit.rs` | 1 | The fit plan comes from the host helper (1 replaced; identical result at the default) | Compact-row tool mode forces every group to its compact level | Not yet (generic switch) |
| `crates/ui-egui/src/dialogs.rs` | 1 | Delegate the existing promotional About block to the new host helper | Retain WordCraft, authors and license while hiding promotion; avoid re-indentation debt | Not yet (generic switch) |
| `crates/ui-egui/src/theme.rs` | 4 | Declare `theme::host` (2 including spacing), resolve the tokens at the single getter (1 replaced), reapply host widget visuals when upstream reapplies appearance (1) | Context-local palette overrides survive every frame and theme change | Not yet (generic theme seam) |

Total: 54 added lines in 13 modified upstream files, including replacements.
Ratchet: 54

The About helper reduces the inherited patch by eight added lines; the lockfile adds one.

The final lane report records the committed measurement and all gate/build results. No upstream
roadmap or Word-parity score is changed: this integration adds an embedding transport and host chrome.

## New files: no upstream patch debt

- `c0/wordcraft-host/`: the page, build configuration and handback documentation.
- `apps/wordcraft-web/src/page_control.rs` and `page_control/{bridge,files}.rs`.
- `crates/ui-egui/src/c0_host.rs` and `c0_host/c0_{files,panels,chrome}.rs`.
- `crates/ui-egui/src/theme/host.rs`.
- `crates/ui-egui/tests/c0_{host_contract,chrome,control_passes,toolmodes}.rs`.

## Couplings without an edited line: re-check on every upstream pull

| Our code | Upstream coupling | Evidence |
|---|---|---|
| `page_control/bridge.rs` | WebRunner event listeners lend AppRunner and `logic()` runs an unpainted pass; the same runner must be passed before startup | Native feature source contract, wasm feature check and guest page build |
| `page_control.rs` | Which methods only read; WordCraft's `ui.text` and other synthetic input acknowledge queueing immediately, unlike PhotoCraft's delayed input protocol | Native feature tests and `c0_control_passes` |
| `c0_host` | `ControlRequest` and `WordApp::with_control`; `logic` drains requests in order | Every host request goes through `control::handle`; unpainted-pass tests use the real queue |
| `c0_host` | Body block indices, resolved paragraph styles, zero-based outline levels, Title style and `Session::rev()` | Outline, revision, hostile block and navigation tests |
| `c0_host` | `caret.set`, `nav_to_caret`, `scroll_to_caret` and `want_focus` | Same policy as the Navigation pane; caret/scroll contract test |
| `c0_host/c0_files.rs` | `WordApp::run` and `ui.saveChanges` retain the normal unsaved-changes lifecycle for file.new/file.open, including byte imports | Cancel/Don't Save replacement tests, import format tests and native engine coverage |
| `c0_host/c0_files.rs` | `io::save_bytes_with`, `cmd::file::export_options`, Session password and immutable committed document | Read-only snapshot/reopen and encrypted snapshot tests |
| `page_control/files.rs` | Services inbox/open_async, upstream file filter registry and file count cap; an atomic pending flag makes late chooser answers inert | Native cancellation token test; wasm feature build |
| `c0_host/c0_panels.rs` | Existing view commands, ui.startScreen startup preference and toggle-only ui.collapseRibbon | Per-key report/idempotence tests and no-document mount test |
| `theme/host.rs` | Scalar Color32 token fields and parent's widget visuals derivation; WordCraft currently has no radius tokens | Palette apply/report/clear tests and wasm compile; removed token fields fail to compile |
| Chrome switches | All four promotional surfaces and ui.discord's latent Help KeyTip; license/authorship is separate from the promotional block | Rendered title-bar, Backstage, Help and About text tests, plus direct-command/KeyTip coverage |

The optional page picker tracks only open-picker requests. A native browser-owned chooser cannot
be closed by the page; cancelling its request prevents any later answer from importing a file.
Browser save tickets and the Save Changes prompt keep their existing lifecycle.
| `c0_host/c0_chrome.rs` | `ribbon::show` panel call, `ribbon_fit::begin` plan, `panes::show` right-hand panels, `WordApp::session.view` pane flags and `restrict.open` | Rendered tests per mode; the folded dock hides the right panes by clearing their flags around `panes::show` and restoring them, so a new right pane must be added to `RIGHT` |
