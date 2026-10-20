C0doc fork lane — verified implementation; upstream CI baseline failure recorded
2026-10-20, Linux executor sing

Branch: c0/main
Base/main: d1b0bfc3f9dd83126b162a42b667facecb003b7f
No push performed. No shared branch, lease, claim or scheduler configuration changed.

Commits
8a6c3ded feat(c0doc): add context-local WordCraft theme overrides
550a7f07 feat(c0doc): add default-preserving host chrome switches
d781a22e feat(c0doc): implement host document and settings requests
4c59ff59 feat(c0doc): bridge the existing web editor into a host page
7aa864a4 test(c0doc): cover host contracts chrome and unpainted control passes
Documentation/report commit: docs(c0doc): record verified guest contracts and lane evidence.
Its SHA is the commit containing this report; resolve with:
git log -1 --format='%h %s' -- c0/wordcraft-host/LANE-REPORT.md
The stdout hand-back prints all six commit SHAs. Every commit ends with:
Co-Authored-By: GPT-6.1-sol <noreply@openai.com>

Verification
All heavy work used smoac-slot run heavy 2 3600, CARGO_BUILD_JOBS=4.
Cargo invocations never exceeded two concurrently; final gates ran sequentially.
Native full-suite tests used RUST_TEST_THREADS=4. Release optimizer/Rayon workers
were capped at four. The wrapper owns host capacity and reports a heavy cap of 12;
the requested capacity was not used to modify host policy. The initial spaced
--test arguments caused automatic gate admission; equivalent --test=<name>
arguments admitted the requested heavy route. The waiting duplicate was cancelled.

Final command results (counts are per command, not unique tests):
- cargo test -p wordcraft-ui-egui --test=c0_host_contract --test=c0_chrome
  --test=c0_control_passes: PASS, 21 passed, 0 failed, 0 ignored.
  Breakdown: c0_host_contract 15, c0_chrome 3, c0_control_passes 3.
  These are all c0_* integration test targets in this lane.
- cargo test -p wordcraft-web --features page-control: PASS, 10 passed,
  0 failed, 0 ignored; five new bridge-policy tests and five upstream tests.
- cargo check --target wasm32-unknown-unknown -p wordcraft-web
  --features page-control: PASS, compilation check, 0 tests.
- cargo test -p wordcraft-ui-egui: PASS, 751 passed, 0 failed, 2 upstream
  tests ignored. Breakdown: unit 720 passed/2 ignored, breaks_menu 2,
  c0_chrome 3, c0_control_passes 3, c0_host_contract 15, native_print 8,
  doc-tests 0. No upstream test was edited or weakened.
- cargo clippy -p wordcraft-ui-egui -p wordcraft-web
  --features wordcraft-web/page-control -- -D warnings: PASS, 0 warnings.
- cargo fmt --all -- --check and git diff --check: PASS.
- cargo xtask ci: FAIL at the upstream workspace test step. Formatting and
  workspace Clippy with --all-targets/-D warnings both PASS. Completed test
  binaries report 2305 passed, 1 failed, 1 ignored before Cargo stops.
  Failure: wordcraft-layout, tests_complex::myanmar_prevowel_is_drawn_before_its_consonant,
  crates/layout/src/tests_complex.rs:82; expected glyph 4097, got 4146.
  The layout binary reports 441 passed/1 failed. This is not a fork regression:
  a detached, untouched main worktree at d1b0bfc3 reproduces the identical
  assertion with cargo test -p wordcraft-layout
  tests_complex::myanmar_prevowel_is_drawn_before_its_consonant -- --exact:
  0 passed, 1 failed, 441 filtered. That temporary worktree was removed.
  CI reached this failure within 30 minutes; it was not reported as green.
- Skipped CI parts were run separately: cargo xtask assets PASS (55 attributed
  asset files); cargo xtask layers PASS (20 crates, no violations).
- cargo xtask wasm: PASS, all 16 wasm-safe packages, 0 tests.

CI and the targeted gates ran on the final worktree before the logical commit
series. Partitioning changed the index only, not the validated Rust source.

Known upstream wasm warnings, left unchanged:
- crates/engine/src/cmd/citations.rs:534, unused by_author.
- crates/engine/src/speech.rs:252, unused primary_tag.
- crates/engine/src/speech.rs:260, unused voice_language.

Fixes made during verification
- c0.set now accepts theme:"light"|"dark" and returns the required flat
  {applied,unknown,invalid} report, with coverage for mixed settings and bad keys.
- rfd's file-handle-inner feature is enabled only through page-control. Browser
  picker reads validate Blob size before allocation and handle rejected reads.
- The new chrome text harness acknowledges egui texture deltas. The new queue
  harness waits for the real renderer's wall-clock debounce before asserting
  that reads demand no presentation. No assertion was relaxed.
- Navigation coverage checks a distant caret's actual viewport alignment,
  instead of trying to access a private CanvasState flag.
- community_links=false also blocks the latent Help KeyTip and direct
  ui.discord command; defaults preserve upstream behavior.
- About's promotional block moved behind a new-file helper, reducing its
  upstream added-line debt from nine to one.
- Installed the missing Clippy toolchain component. Trunk required NO_COLOR=true
  because the inherited NO_COLOR=1 is not accepted by its boolean parser.

Release page
Command: trunk build --release in c0/wordcraft-host, under the heavy wrapper.
NO_COLOR=true CARGO_BUILD_JOBS=4 BINARYEN_CORES=4 RAYON_NUM_THREADS=4.
Result: PASS, Rust release compilation 4m17s; complete Trunk build about 10m31s.
dist/c0host/wordcraft-web-d26fb7859ffc4c55_bg.wasm: 44322356 bytes.
Total dist/c0host: 44491211 bytes, 3 files (sum of file lengths).
cat /proc/loadavg immediately before build:
12.00 15.54 16.62 9/1719 4045015
cat /proc/loadavg immediately after build:
18.94 18.01 17.32 18/2156 1015922
This is shared-host load, not an isolated performance benchmark.

Runtime and visual checks
- The native ui_shot example captured /tmp/c0doc-light.png,
  /tmp/c0doc-dark.png and /tmp/c0doc-about.png. All three were viewed.
  Title/community chrome is hidden, both host palettes render correctly,
  the status bar remains, and About retains WordCraft, authors and license.
- A headless Chromium/CDP smoke test served the built dist directly. All seven
  c0.* methods succeeded after future requestAnimationFrame scheduling was
  disabled. It imported text, navigated, exported DOCX, validated the set and
  inspect shapes, and confirmed the dark theme and hidden chrome.
  After disabling presentation scheduling: set 40.6ms, newDocument 341.7ms,
  openFile 96.8ms, goToBlock 6.1ms, documentBytes 54.7ms,
  cancelFileDialog 6.3ms, final inspect 3.4ms. These are smoke timings only.

Final request/reply contract
window.__c0hostRequest aliases window.__wordcraftControl. JSON string in,
Promise of JSON string out. Every success is {ok:true,result:...}; every
failure, including malformed transport input, is {ok:false,error:string}.

c0.inspect {} -> {
  title:string, dirty:bool, revision:number, pages:number, words:number,
  canUndo:bool, canRedo:bool,
  outline:[{level:number,text:string,block:number}],
  panes:{ruler:bool,navigation:bool,styles:bool,comments:bool},
  ribbonCollapsed:bool, startScreen:bool, theme:"light"|"dark"|"system",
  titleBar:bool, communityLinks:bool, fileDialog:bool
}.
revision is Session::rev(). Title has outline level 0; Heading 1..9 use 1..9.
block is the zero-based top-level body block index. No-document pages/words=0.

c0.goToBlock {block:number} -> {block:number}.
Moves the caret and uses the Navigation pane's focus/scroll policy. Bad or
out-of-range indices return the error envelope.

c0.documentBytes {format?:"docx"} -> {
  name:string,format:"docx",bytes:string,revision:number
}.
bytes is standard padded base64, without a data URL prefix. Uses the ordinary
DOCX writer, export policy and document password. Does not mark saved, mutate
the document/history/path/revision, or commit a comment draft.

c0.openFile {name:string,bytes:string} -> {
  path:string,paragraphs:number,words:number,encrypted:bool
}, or {pending:"saveChanges"} when replacement needs the normal unsaved-work
prompt. The document is retained until ui.saveChanges resolves that prompt.
All importer formats use the existing File > Open path. Malformed imports and
encrypted imports requiring a password return errors; this host method supplies
no password and opens no password dialog.
name must be a basename of at most 255 UTF-8 bytes. The decoded file limit is
72 MiB; c0.* JSON requests are also bounded to 96 MiB (other requests: 1 MiB).

c0.newDocument {template?:string} -> {anchor:Pos,focus:Pos}, or
{pending:"saveChanges"}. Pos is {story:"body",path:[number,...],off:number}
for the new document's body selection. Default template is blank. Existing
file.new owns template names/fallback; this exits Backstage immediately.

c0.cancelFileDialog {} -> {cancelled:bool}.
Retires an open-picker request/app continuation. Repeated cancellation is
false. A late chooser answer is inert; the page cannot close browser-owned
native chooser chrome. Save tickets and unsaved-changes prompts are separate.

c0.set {theme?:"light"|"dark",themeTokens?:object|null,panels?:object}
-> {applied:[string],unknown:[string],invalid:[string]}.
Entries identify theme, themeTokens.<field>, panels.<field>, or an unknown
top-level key. Known keys with bad values are invalid; unknown keys are unknown.
Good siblings apply. Malformed containers return an error before any change.
Example result for theme dark, a valid accent and hidden title_bar:
{applied:["theme","themeTokens.accent","panels.title_bar"],unknown:[],invalid:[]}.

Boolean panel keys: title_bar, community_links, ribbon_collapsed, start_screen,
ruler, navigation_pane, styles_pane, comments_pane. status_bar is unknown.
Pane switches reuse view.*; title/community switches default true, use optional
boolean ui.titleBar/ui.communityLinks setters, and are not serialized to prefs.
startScreen/start_screen is the existing startup preference, not Backstage visibility.

themeTokens replaces the complete context-local override set; {} or null clears it.
Colors are #rrggbb or #rrggbbaa. Accepted names are WordCraft Tokens field names:
title_bar ribbon tab_strip canvas panel border border_strong text text_dim
text_disabled icon hover pressed checked accent accent_text on_accent input
input_border group_label ruler ruler_margin ruler_tick status_bar selection
find_highlight page_shadow handle handle_shadow guide crop_handle crop_dim
balloon markup_area markup_edge resolved_text menu blue red green orange
close_hover close_pressed on_close.
All 44 scalar color fields are supported. WordCraft has no radius tokens.
The dark metadata field and non-scalar color arrays are not color overrides.

Final upstream patch debt (added column, not net lines)
git diff --numstat --diff-filter=M main HEAD -- . ':!c0'
Cargo.lock: 1
apps/wordcraft-web/Cargo.toml: 3
apps/wordcraft-web/src/main.rs: 2
apps/wordcraft-web/src/web.rs: 7
crates/ui-egui/Cargo.toml: 1
crates/ui-egui/src/backstage.rs: 3
crates/ui-egui/src/chrome.rs: 4
crates/ui-egui/src/control.rs: 1
crates/ui-egui/src/dialogs.rs: 1
crates/ui-egui/src/lib.rs: 13
crates/ui-egui/src/ribbon.rs: 3
crates/ui-egui/src/theme.rs: 4
Total: 43 added lines in 12 upstream files. New files and c0/ are excluded.
Ratchet: 43
PATCH-DEBT.md records the same rows, their reasons and upstream coupling points.

Assumptions I could not confirm
- The real C0VIBE/C0X embedding and its interception of new windows were not run;
  the built guest was tested in standalone headless Chromium.
- The host's intended meaning of startScreen and Title's outline level was not
  independently confirmed: startup preference and level 0 follow this lane's contract.
- Headings nested in tables, text boxes, headers or footers are not integer
  top-level body outline entries. Host use of these structures was not tested.
- Browser-owned chooser interaction/late native picker answers were not driven
  in Chromium; retirement is covered by native cancellation tests and wasm build.
- ../../craftrules/AGENTS.md is absent on this executor. No Microsoft Word
  side-by-side comparison or additional native OS execution was performed here.

Evidence logs remain under /tmp/c0doc-*: final-{a,b,c,d,e,fmt}.log, ci.log,
upstream-baseline.log, assets.log, layers.log, wasm.log, f2.log, shot.log,
browser3.log. No source tests were weakened and no upstream behavior at default
switches was intentionally changed. No proprietary assets were added.
