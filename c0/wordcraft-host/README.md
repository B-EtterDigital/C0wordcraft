# C0doc: WordCraft as a C0VIBE guest page

This page builds the existing `apps/wordcraft-web` binary with the opt-in `page-control`
feature. It adds no alternate editor, command implementation, JavaScript bundle or assets.
The body and canvas are transparent until WordCraft paints. The host draws its loading state,
title and surrounding chrome. WordCraft retains its document status bar.

Build from this directory on the executor:

```sh
smoac-slot run heavy 2 3600 -- env CARGO_BUILD_JOBS=4 trunk build --release
```

Trunk writes `../../dist/c0host`, hashes its artifacts and uses the relative public URL `./`.
Serve that directory over HTTP(S); retain the relative paths if relocating it. The emitted
Trunk JavaScript is generated from Rust, not maintained here. No proprietary assets are added.

## Page function and envelopes

As in the sibling PhotoCraft page, the only handwritten JavaScript aliases the bridge with a
getter. `window.__c0hostRequest` is undefined until WordCraft installs `window.__wordcraftControl`.
After that, both names refer to the same function. Send JSON text and await JSON text:

```js
const reply = JSON.parse(await window.__c0hostRequest(JSON.stringify({
  method: 'c0.inspect', params: {}
})));
```

Every reply is `{"ok":true,"result":...}` or `{"ok":false,"error":"..."}`. Malformed JSON,
non-string input, missing methods, invalid params and stopped channels also use the error
envelope. Missing/null params become `{}`; supplied params must be an object. No request id or
socket authentication key is needed: the function is a capability inside the guest page.
Only a trusted same-origin embedder should be able to call it. The host polls; there are no events.

All methods go through `WordApp::with_control` and WordCraft's existing `control::handle`.
A web build without `page-control` exposes neither bridge function nor alternate picker behavior.

## When a request is answered

The bridge runs an egui pass **inside the call**, through the same `WebRunner` event-listener
mechanism as PhotoCraft. The pass drains WordCraft's control channel and lays out the app without
painting. The answer is available without waiting for a presented frame, including a guest that
is not currently being presented. The function returns a Promise resolving to the reply string.

`c0.inspect` and `c0.documentBytes` only read and do not request presentation. The ordinary read
methods `ui.inspect`, `document.inspect`, `ui.parity`, `engine.commands` and execution of
`document.inspect`/`format.state` do the same. Other calls request a repaint after the pass.
WordCraft acknowledges synthetic input when queued; the bridge runs bounded extra passes to play
it before returning. Delayed replies get additional passes while the app asks for repaint.
`ui.screenshot` needs rendered pixels and still waits for presentation. Before the runner is ready,
the bridge falls back to waking the app and waiting. There is no transport deadline; callers own
their timeout policy. Native `c0_control_passes` tests prove the one-pass queue behavior; the web
feature tests and wasm check cover the browser integration's compilation.

## Host methods

Names and reply keys are case-sensitive. Block indices are zero-based indices in the document's
body, not page numbers, and can change after edits. Refresh the outline when `revision` changes.

| Request | Params | `result` |
|---|---|---|
| `c0.inspect` | `{}` | `{title, dirty, revision, pages, words, canUndo, canRedo, outline, panes, ribbonCollapsed, startScreen, theme, titleBar, communityLinks, fileDialog}` |
| `c0.selection` | `{}` | `{kind:"text",text,start,end,paragraph,documentTitle,wordCount}` with a selection (`start`/`end` are character offsets in the body text, paragraphs joined by a line break; null outside the body), `{kind:"caret",paragraph,documentTitle,wordCount}` without; read only |
| `c0.place` | `{text, mode:"replace"\|"insert"\|"after"}` or `{png\|jpeg: base64, name?, widthPx?}` | `{placed:"text",mode,requestedMode,chars,paragraphs,revision}` or `{placed:"image",name,bytes,widthPx,revision}`; one undo step, marks the document dirty; a picture over 40 MB, bad base64 or an unknown mode answers `{ok:false,error}` |
| `c0.goToBlock` | `{block}` | `{block}`; moves the caret to that body paragraph or a table's first paragraph, focuses the page and uses the Navigation pane's scroll policy |
| `c0.documentBytes` | `{format?: "docx"\|"pdf"}` | `{name, format, bytes, revision}`; a base64 snapshot using File > Save's writer for that format (DOCX with export options and document password; PDF from the engine's own PDF writer, never encrypted) |
| `c0.openFile` | `{name, bytes}` | The normal `file.open` result, normally `{path, paragraphs, words, encrypted}`, or `{pending:"saveChanges"}` |
| `c0.newDocument` | `{template?: string}` | `{anchor,focus}` (WordCraft selection positions), or `{pending:"saveChanges"}`; blank by default, closes Backstage, never opens a start screen |
| `c0.cancelFileDialog` | `{}` | `{cancelled}`; false if no picker request or app file-dialog continuation was pending |
| `c0.set` | `{theme?:"light"\|"dark", themeTokens?, panels?}` | `{applied,unknown,invalid}`; arrays contain `theme`, `themeTokens.<field>`, `panels.<field>`, or an unknown top-level key |

`title` is WordCraft's title stem. `dirty` is `Session::dirty`; `revision` is exactly `Session::rev()`.
`pages` is the laid-out page count and `words` the document model's word count, both zero with no
document open. `outline` contains nonempty body paragraph headings in document order:
`{level,text,block}`, where Title is level 0 and Heading 1–9 are levels 1–9. Resolved outline styles
are supported; TOC styles are excluded. Headings inside tables, text boxes, headers and footers
are not exposed as integer body paragraph entries. `panes` is `{ruler,navigation,styles,comments,toolbar,toolbar_double,toolbar_floating,right_dock_collapsed}`.
`startScreen` reports the startup preference; `fileDialog` reports an outstanding picker request.
`theme` is `light`, `dark` or `system`. The extra `titleBar` and `communityLinks` fields report the
two host chrome switches, which are intentionally omitted from persisted preferences.

File payloads use standard padded base64 without a data URL prefix. `name` must be a basename,
at most 255 UTF-8 bytes, with no slash, backslash or NUL. There is no disk/path API here.
The importer is WordCraft's own, including DOCX/DOCM/DOTX/DOTM, legacy DOC/DOT, ODT, RTF,
TXT, Markdown, HTML, TeX, JSON and supported XML documents. Unreadable files and encrypted files
requiring a password return errors before any replacement prompt; no password dialog is opened.
An empty file follows File > Open's blank-document behavior.

Open/New run through the application's normal unsaved-changes path, including comment drafts
and outstanding browser saves. A dirty document stays intact and returns `pending:"saveChanges"`.
The host can let the app show Save / Don't Save / Cancel, or answer through
`engine.execute {command:"ui.saveChanges",params:{answer:"save"|"dontSave"|"cancel"}}`.
A save that is cancelled or fails does not replace the document. The transport never creates
an OS file picker for Open/New. Template names and defaults belong to `file.new`: current built-ins
are blank, sample, letter, resume and report; an unrecognized name follows upstream's blank fallback.

`documentBytes` is read-only: it does not stamp a save, clear dirty, alter history, path or revision,
or commit a comment draft. It serializes the committed document. Bytes encrypted by File > Save's
password policy retain that encryption; `c0.openFile` does not supply passwords.
Files are capped at 72 MiB decoded; `c0.*` JSON text at 96 MiB. Other requests keep 1 MiB.
The page picker uses the same upstream file-count cap, and bounds pending file bytes to 72 MiB.

`cancelFileDialog` retires the current in-page open-picker request and any app file-dialog
continuation. A browser-owned chooser window may remain on screen: the page cannot close native
browser UI, but a late answer is ignored and cannot import a file. The request is no longer pending.
It does not answer or dismiss the unsaved-changes prompt and does not cancel a browser save ticket.
A valid `c0.openFile` or `c0.newDocument` supersedes an older open-picker request in the same way.

## Chrome settings

The four mode keys are held in the page, never saved to preferences. `c0.set {panels:{...}}` accepts the following boolean keys. Each known valid key is in `applied`;
unknown names are in `unknown`, known names with non-boolean values in `invalid`. A bad key does
not prevent good siblings from applying. A malformed container returns an error before changes.
Setting the same value twice is idempotent, including `ribbon_collapsed`.

| Key | Meaning / implementation |
|---|---|
| `title_bar` | Show the custom title bar row; default true. Also available as `ui.titleBar {value?:bool}` (absent toggles) |
| `community_links` | Show the title bar Discord pill, Backstage Discord footer, Help Community Discord and About's promotional description/links; default true. `ui.communityLinks {value?:bool}` (absent toggles) |
| `ribbon_collapsed` | Existing ribbon collapse state, through `ui.collapseRibbon` only when different |
| `start_screen` | Existing startup preference, through `ui.startScreen`; call `c0.newDocument` to enter a blank document immediately |
| `ruler` | `view.ruler` |
| `navigation_pane` | `view.navigationPane` |
| `styles_pane` | `view.stylesPane` |
| `comments_pane` | `view.commentsPane` |
| `toolbar` | The ribbon body shown (default true); false is the ribbon collapsed to its tabs, the inverse of `ribbon_collapsed`. The tab strip is never removed |
| `toolbar_double` | Compact single row: every ribbon group at its compact level (small buttons without labels, shorter Styles gallery), collapsing to drop-downs only when even that is too wide. Default false |
| `toolbar_floating` | The ribbon body in a movable strip over the page (drag its grip), tabs stay docked. Default false; combines with `toolbar_double` |
| `right_dock_collapsed` | Fold the right-hand panes (Styles, Style Inspector, Accessibility, Alt Text, Restrict Editing, Comments) to a 34-point icon strip; an icon opens its pane and unfolds, as does opening any pane by command. Panes stay open while folded. Default false |

No `status_bar` switch is added: it is reported as unknown. About always retains WordCraft's name,
version, authors/copyright and MIT OR Apache-2.0 license. Existing `show_discord` still independently
controls the title-bar pill; the host switch gates all the community entries. The hidden community
switch also blocks `ui.discord`, including its latent Help KeyTip. Switch defaults leave the
standalone interface unchanged. Link handling remains `ctx.open_url`; the host can catch new
windows with its usual browser embedding policy.

## Runtime theme tokens

`c0.set {themeTokens:{tokenName:"#rrggbb"|"#rrggbbaa"}}` replaces the complete override set.
An empty object or `null` clears it. `c0.set {theme:"light"|"dark"}` selects the preset
through `ui.theme` and applies it immediately, including when no document is open.
Overrides live only in this egui context and are applied over
the preset at the single `Tokens::get` seam, including after theme changes. Egui widget visuals
receive the same colors for both style branches. Send the host's active light/dark palette again
when that palette changes; no second theme or persistence system is introduced.

Scalar color names accepted from `theme::Tokens`:

```text
title_bar ribbon tab_strip canvas panel border border_strong text text_dim text_disabled
icon hover pressed checked accent accent_text on_accent input input_border group_label
ruler ruler_margin ruler_tick status_bar selection find_highlight page_shadow handle
handle_shadow guide crop_handle crop_dim balloon markup_area markup_edge resolved_text
menu blue red green orange close_hover close_pressed on_close
```

Unknown names are returned in `unknown`. Known names with malformed colors are in `invalid` and
fall back to the preset; valid siblings still apply. The leading `#` is required; alpha is straight
RGBA on input and converted by egui. WordCraft has no radius tokens, so radius keys are unknown.
`dark` is preset metadata and the author/leader color arrays are not scalar color tokens.

## Ordinary WordCraft control methods

All existing requests still work through the page function. See
[`docs/control-protocol.md`](../../docs/control-protocol.md) for parameter details and command ids.
Each uses the same success/error envelope described above.

| Method | Result / purpose |
|---|---|
| `engine.execute` (alias `command`) | Existing command result; params `{command,params?}` (or `id` for the alias) |
| `engine.commands` | Array of command descriptors, enablement and params; optional search |
| `document.inspect` | Existing engine document structure |
| `ui.inspect` | UI/preferences, view, dialogs, geometry, caret, title, dirty and timing |
| `ui.parity` | Engine feature-catalog parity |
| `ui.click`, `ui.clickText`, `ui.move`, `ui.press`, `ui.release`, `ui.drag`, `ui.key`, `ui.wheel`, `ui.text` | Acknowledgement that input is queued; `clickText` also reports the screen position |
| `ui.render` | Page PNG as base64 (browser callers omit `path`); page/scale optional |
| `ui.screenshot` | Window PNG as base64 when supported by the runner; requires presented pixels |
| `ui.resize`, `ui.focus` | Existing viewport request acknowledgement; browser viewport behavior is browser-owned |
| `app.quit` | Existing close request acknowledgement; an embedder owns guest lifetime |

## Branches, tests and upstream pulls

`main` mirrors upstream at `d1b0bfc3f9dd`; only `c0/main` receives these commits. Never push from this
lane. `PATCH-DEBT.md` lists every modified upstream file and counts the exact added-line ratchet.
All substantial host implementation and tests live in new files. No upstream progress/parity
claim is raised by this embedding integration.

Run each command under `smoac-slot run heavy 2 3600 -- env CARGO_BUILD_JOBS=4`;
if heavy is refused, use `smoac-slot run gate 3 3600`. At most two Cargo invocations
may run at once. The slot wrapper may require gate admission for verification too.

```sh
cargo test -p wordcraft-ui-egui --test c0_host_contract --test c0_chrome --test c0_control_passes --test c0_toolmodes
cargo test -p wordcraft-web --features page-control
cargo check --target wasm32-unknown-unknown -p wordcraft-web --features page-control
cargo xtask ci
```

The feature module is compiled in native tests as well as wasm (K-41). Its browser half is covered
by the wasm check and page build. Rendered text tests cover each community entry and title-row
suppression; lifecycle tests use the actual app commands and control queue. `LANE-REPORT.md`
records the final evidence, artifact sizes, host load and assumptions for this lane.
