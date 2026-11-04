# Validation and limits

Local validation on 1 November 2026 used Rust 1.99.1, Windows MSVC, Linux x86_64 and `wasm32-unknown-unknown`.

## Checks completed

| Check | Result |
| --- | --- |
| Linux `cargo build --locked`, `cargo test --locked`, Clippy across all native targets with warnings denied and formatting | Passed |
| Windows release build, native tests, Clippy across all native targets with warnings denied and formatting | Passed |
| Native tests | 20 contract tests, 10 drawing tests, 8 video and camera sampling tests, 5 lettering tests, 6 CLI integration tests, 5 CLI download tests and 2 documentation examples passed |
| Library example | Compiled and ran successfully |
| WebAssembly release build and Clippy with warnings denied | Passed with the `web` feature and default features disabled |
| wasm-bindgen | Generated browser bindings using pinned CLI version 0.2.130 |
| Editor tests in a headless browser | 16 `wasm-bindgen-test` cases passed in Chromium 156 headless shell on Windows; each of 9 deliberate defects (typing runs committed per key, a selectable onion layer, onion skin during playback, no diagram junctions, pointer cells off by one, no stroke preview, a swallowed stamp error, CRLF kept on import, unapplied edits ignored) failed at least one case |
| GitHub workflow syntax | Passed actionlint |
| Aikido source scan | Zero issues across 15 files; zero skipped paths |
| Aikido dependency scan | Reachable weezl alert fixed; 5 remaining transitive alerts at that scan, 4 of them since removed with `figlet-rs`; see [dependency notes](DEPENDENCIES.md). Rescanned with the HTTPS client crates on 1 November 2026, no new alerts |
| GIF decoder fuzzing | 1,027,670 executions, no crash, timeout or out-of-memory failure: cargo-fuzz 0.13.2 with libFuzzer and AddressSanitizer on Windows MSVC, 6 workers for 15 minutes each, inputs up to 4 KB, from 15 seeds (11 real GIFs, 4 hand-built) to a 3,799-input corpus |

Contract tests cover every control/non-ASCII byte, trailing spaces and rectangular parsing, drawing and transforms, all 95 printable characters across the 6 styles, original templates and scenes, deterministic seeds, per-frame timing, JSON validation and round trips, GIF timing/truncation/preflight bounds, GIF decoding (transparency, disposal, interlacing, palettes, repeated clear codes and malformed LZW or block structure), image ramp modes, HTML injection strings and CLI output protection.

The fuzz target ran `src/gif.rs` directly, with the preflight's screen limit (at most 4096 x 4096 and 16,777,216 pixels) and the 64 million pixel budget across frames. The crate's `cdylib` would not link under the sanitiser, so `inspect_gif`'s frame-count and rectangle checks were outside the target; the decoder checks frame rectangles itself. Inputs over 4 KB and screens near the 4096-pixel limit with many frames were not fuzzed. Changes to `src/gif.rs` since the run touched only its tests.

Effect tests run all 11 effects on catalogue art, block lettering, single characters and a blank frame at 4 duration and FPS settings. They check the frame count, equal dimensions, printable ASCII, determinism, seed use, the final frame and the duration, FPS and cell limits. Catalogue tests check at least 60 uniquely named pieces, valid categories and tags, size, trimmed edge rows and tag search. CLI tests cover `list --tag`, effects from files and stdin, batch conversion, collision and overwrite refusal, partial failure reporting and plain-HTTP refusal. Download tests use a loopback test server, never the internet, for image content, a non-image content type, oversized bodies with and without a length, and a timeout.

Browser checks for the effects release used the in-app browser over local HTTP: all 11 effects applied to the selected frame and undone, an over-budget effect refused with a visible error, catalogue search by tag with grouped results and an empty-result message, the timeline reaching the unchanged final frame, no canvas, SVG or image elements and no console errors. On the deployed editor, the keyboard alone opened the effect panel, chose an effect, ticked reverse and applied it, and chose a source, searched the catalogue and picked a piece, with focus moving in the expected order and visibly outlined on the panel summary and Apply button. The search result count is now a polite live region; its announcement has not been tested with a screen reader.

On 1 November 2026 a Linux x86-64 build of 7728d14 (Arch Linux, Rust 1.99.1) passed `cargo test --locked`, and the binary from `cargo build --locked --release` listed catalogue tags, made a 5-frame, 500 ms effect, refused to overwrite its output, converted an `https://` GIF to 44 frames, and refused an HTML page and a plain `http://` URL. Every frame was printable ASCII.

Independent code reviews found defects in JSON allocation, frame line allocation, GIF rectangles, font markers, ambiguous stdin, asynchronous edits, empty-frame editing and preview overflow. They were repaired and checked. The malformed-font, GIF-rectangle and duplicate-project-field tests reproduced their failures before repair; browser probes likewise reproduced editable-during-load, empty-edit rejection and negative tall-preview positioning before repair.

## Lettering reference checks

On 1 November 2026 the renderer was compared with figlet 2.2.5, built from cmatsuoka/figlet `202a0a8` on Linux, and figlet.js 1.12.0, using the bundled font files:

- Horizontal layout and word wrap: 1,260 of 1,260 cases matched figlet exactly (21 fonts, 5 texts with smushing-rule characters, `-s`, `-W`, `-k` and `-S`, no wrap and wraps of 30 and 45 columns).
- Vertical layout: 185 of 185 cases matched figlet.js with full-width horizontal layout (13 fonts, 5 multiline texts, full, fitted, smushed and default vertical layout). The 20 excluded default-layout cases are the documented specification difference in [the format notes](FORMAT.md). figlet.js fitting keeps the first glyph's leading blank columns where figlet drops them, so fitted horizontal layout was compared only against figlet.
- The fixtures in `tests/fixtures/lettering` come from those 2 programs, and `tests/lettering.rs` checks each layout mode against them.

The browser editor listed all 21 fonts and the 2 styles, generated wrapped, vertically smushed lettering as printable DOM text, accepted an ASCII `.tlf` upload and refused a `.tlf` with a non-ASCII glyph, naming the glyph and byte. Browser runtime errors: zero.

## Actual browser checks

Headless Brave loaded and executed the generated WebAssembly module over local HTTP. Checks passed for:

- Printable DOM text, text selection and absence of canvas/image/SVG artwork elements.
- Reduced-motion paused start, play/pause and frame stepping.
- All 12 static templates, 6 lettering styles and 8 procedural scenes.
- Drawing tools in the Chromium-based Claude Browser pane over local HTTP: pencil, eraser, line, rectangle, fill, typing, text stamps, and area select, move, cut, copy, paste (including text from the system clipboard) and clear. Diagram boxes, labels, connectors and arrowheads joined at corners and crossings. Every tool was also used by keyboard alone, and each step was undone and redone. Onion skin filled only blank cells, stayed out of text selection and hid during playback. A 375-pixel layout had no page-level horizontal overflow, controls at least 44 pixels high and pointer drawing on the grid.
- Frame editing, padding, timing, duplication, reordering, flips, add/delete, undo/redo and clearing a frame.
- Rejected Unicode and protection of unapplied edits.
- Pending file reads disabling draft editing and failed imports preserving the current artwork.
- Text, JSON and standalone HTML downloads, JSON reimport and execution of the standalone player.
- CRLF/ragged text import, custom FLF import, image conversion and 2-frame GIF import retaining 20/70 ms delays.
- Layouts at 360, 768 and 1,440 pixels, no page-level horizontal overflow and visible buttons at least 44 pixels high.
- Reachable top rows in a 256-row preview. Browser runtime errors: zero.
- Two complete non-looping playback cycles in the Rust editor and standalone HTML player, preserving 80/100 ms frame durations under a controlled browser clock, plus pause/resume retaining the remaining frame time.
- An actual unload prompt after a deliberately blocked JSON download. Download requests retain unsaved-work protection.

Screenshots were visually inspected. Clipboard permission was denied by the headless browser even after a test permission request. The visible select-and-copy fallback and actual text selection were checked; a successful clipboard API write was not verified in that environment.

## Automated editor tests

`src/web/tests.rs` embeds the body of `web/index.html` and `web/style.css` (not the page's loader script), starts the editor and drives it only through DOM events and controls. Pencil, eraser, line (including its live preview), rectangle, fill, typing, and area select and move are each used by pointer and by keyboard. Stamped text and the area buttons are clicked; area copy, cut and paste dispatch the clipboard events a browser raises for Ctrl+C, Ctrl+X and Ctrl+V. It also covers diagram boxes, labels and connectors joining with `+`; onion skin staying out of the frame, the frame editor and selected text, and hiding during playback; undo and redo of a stroke and of a typing run by button and shortcut; opening ragged text and JSON projects through the file control; timeline seeking, frame editing, reordering, flipping and deletion; applying an effect; catalogue search; and Unicode or control characters never reaching `#art`. Every read of `#art` checks that it holds printable ASCII in equal-width LF rows. CI runs the suite in headless Chrome on Linux for every pull request.

The suite dispatches synthetic events, so it does not exercise the system clipboard, real pointer capture, touch hardware, the camera, video decoding, downloads, layout at other widths or screen readers. The manual checks above remain the evidence for those. The suite has not been run in Firefox or Safari.

Art stamp tests capture and deduplicate selections, choose saved stamps, replace and cancel previews, place by pointer and keyboard, clip at the grid edges, choose transparent or overwriting spaces and undo or redo a placement as one step. They check the entry and aggregate cell limits, deletion without changing artwork and the empty library message. Mirror tests check that literal flips remain the default and directional mirrors are optional and undoable. Native contracts check both mirror operations twice across all 95 printable characters, retaining dimensions and significant spaces.

## Manual check limits

The manual browser checks above were local session checks. Firefox, Safari, touch hardware, screen-reader behaviour and macOS binaries have not been tested. Windows emits a Cargo PDB-name collision warning because the native library and CLI share a normalised crate name; the checks still pass. The local compiler-cache wrapper could not launch one large WebAssembly compile command, so local WASM checks were rerun successfully with that wrapper disabled.

## Video and camera checks

On 1 November 2026 the editor ran in the Claude desktop Browser pane (Chromium) over local HTTP with a 3-second, 320 x 240, 25 fps VP8 WebM test clip generated by FFmpeg:

- 10 fps at 60 columns gave 30 distinct 60 x 23 text frames of 100 ms each; 12 fps from 0.5 to 1.5 seconds gave 12 frames of 83 or 84 ms summing to 1,000 ms.
- An end time beyond the clip, a capture over the 8,000,000-cell budget and an undecodable file were refused with messages, leaving the project unchanged. Cancelling part-way through a 180-frame conversion left the project unchanged and re-enabled the controls.
- The progress row and controls fitted at 360 and 1,280 pixels wide without horizontal overflow; buttons were 44 pixels high.
- No external requests were made and no console errors appeared. All preview text was printable ASCII.

The machine has no camera, and the editor correctly reported 'No camera was found.' The camera path was then exercised by replacing `getUserMedia` in the page console with a 30 fps stream captured from a hidden test canvas. The live text preview updated; a 2-second recording at 10 fps gave 20 frames summing to 2,000 ms, the camera turned off and its track ended. Turning the camera off during a recording, a `pagehide` event and cancelling a pending permission request each ended the track. With `play()` held unresolved, turning the camera off or a `pagehide` event still ended the track. A `pagehide` whilst permission was pending, followed by `pageshow` and a granted stream, left the camera off with that track ended. Cancelling during a seek that never completed released the video within about 100 ms. A real camera, the browser permission prompt and `visibilitychange` on an actually hidden tab were not verified.

## Deliberate limits

The Rust code contains no video decoder: the browser decodes video, so supported formats vary by browser, and the CLI has no video or camera input. Only the CLI fetches media over the network, from `https://` image URLs. There is no ANSI-colour artwork mode, raster/vector artwork exporter or Unicode-art mode. GIF is the animated image import format; other image inputs use the image library's still-image decoding. Oversize inputs are refused. Browser work is synchronous Rust after the bounded file read, so large valid conversions may briefly occupy the UI thread. Resource limits bound accepted files and projects, not the browser process's total memory.

The browser keeps 100 undo steps within a 16 million cell budget and has no automatic local persistence. A drag, a keyboard stroke or a run of typed characters is one step. Onion skin shows only the previous frame, so the first frame has none. On screens up to 760 pixels wide the drawing options start folded. Drawing tools disable touch scrolling over the grid; choose Select text to scroll it by touch. Save a JSON project to retain all frames and timing. Text export saves one selected frame. Closing or reloading a modified project requests the browser's unsaved-work prompt; downloads are reported as requested because the page cannot prove where the browser saved a file. Requesting a download does not clear the unsaved-work warning.

Calvino design review completed with verified GPT-6 and an Extra High fallback after the model picker showed Pro unavailable. Final publication review and hosted CI/deployment results are recorded in the publication handoff; local checks alone do not establish those external results.
