<p align="center">
  <picture>
    <source media="(max-width: 600px) and (prefers-color-scheme: light)" srcset=".github/assets/banner-phone-light.svg">
    <source media="(max-width: 600px)" srcset=".github/assets/banner-phone.svg">
    <source media="(prefers-color-scheme: light)" srcset=".github/assets/banner-light.svg">
    <img src=".github/assets/banner.svg" width="100%" alt="ApollinaireASCII: Static art, lettering and animation in real ASCII text. Pixel art of a block letter A over a crimson cursor beside the ApollinaireASCII wordmark in slab-serif block letters.">
  </picture>
</p>

<div align="center">

[![Checks and browser](https://github.com/EquityBot/ApollinaireASCII/actions/workflows/checks.yml/badge.svg?branch=master)](https://github.com/EquityBot/ApollinaireASCII/actions/workflows/checks.yml) [![Codecov](https://codecov.io/gh/EquityBot/ApollinaireASCII/graph/badge.svg)](https://codecov.io/gh/EquityBot/ApollinaireASCII) [![CodeQL](https://github.com/EquityBot/ApollinaireASCII/actions/workflows/codeql.yml/badge.svg?branch=master)](https://github.com/EquityBot/ApollinaireASCII/actions/workflows/codeql.yml) [![Snyk](https://github.com/EquityBot/ApollinaireASCII/actions/workflows/snyk.yml/badge.svg?branch=master)](https://github.com/EquityBot/ApollinaireASCII/actions/workflows/snyk.yml) [![SonarCloud](https://sonarcloud.io/api/project_badges/measure?project=EquityBot_ApollinairesASCII&metric=alert_status)](https://sonarcloud.io/summary/new_code?id=EquityBot_ApollinairesASCII)<br>
[![OpenSSF Scorecard](https://raw.githubusercontent.com/ryanduguid/.github/badges/EquityBot/ApollinaireASCII.svg)](https://scorecard.dev/viewer/?uri=github.com/EquityBot/ApollinaireASCII)<br>
[![release](https://badgen.net/github/release/EquityBot/ApollinaireASCII/stable?label=release&color=66023C&labelColor=000000)](https://github.com/EquityBot/ApollinaireASCII/releases/latest) [![crates.io](https://badgen.net/crates/v/apollinaire-ascii?label=crates.io&color=66023C&labelColor=000000)](https://crates.io/crates/apollinaire-ascii) [![rust: 1.99+](.github/assets/badges/rust.svg)](https://www.rust-lang.org/) [![Licence: MIT](.github/assets/badges/licence.svg)](LICENSE.md) [![Editor: in browser](.github/assets/badges/editor.svg)](https://equitybot.github.io/ApollinaireASCII/)

[What it does](#what-it-does) · [Run the CLI](#run-the-cli) · [Use the library](#use-the-library) · [Browser editor](#build-the-browser-editor) · [Alternatives](#when-to-choose-something-else) · [Licence](#foundation-and-licence)

</div>

![A ghost drawn in # and : characters floats up and down](docs/media/ghost.gif)

<sub>A preview of `apollinaire animate ghost`. The tools themselves output text, never images.</sub>

Static art, block letters and animated scenes made of real, selectable ASCII characters. The `apollinaire-ascii` Rust library and CLI share their rendering and validation code with the browser editor.

[Open the browser editor](https://equitybot.github.io/ApollinaireASCII/) · [CLI guide](docs/CLI.md) · [Library example](examples/library.rs) · [API docs](https://docs.rs/apollinaire-ascii) · [crates.io](https://crates.io/crates/apollinaire-ascii) · [Research and provenance](docs/RESEARCH.md)

```text
       .-.
      (o o)
      | O \
       \   \
        '~~~'
```

Every frame contains bytes 32 through 126. Line feeds separate rows when saved as text. The browser displays artwork in a `<pre>` using `textContent`. There are no image, SVG, canvas, Braille or Unicode block artwork outputs. Image and video files and the browser camera are accepted as conversion inputs.

## What it does

| Area | Included |
| --- | --- |
| Static art | 89 original pieces in 7 categories (animals, nature, objects, characters, borders and dividers, emoticons, banners), searchable by name or tag; blank canvases, character editing, borders, crops, flips and clipped composition |
| Lettering | 21 FIGlet and TOIlet fonts plus filled block and outline styles; font-default, full-width, fitted and smushed layout for letters and lines; word wrap; multiline text; alignment; validated ASCII-only `.flf` and `.tlf` import |
| Animation | Ghost, rotating donut, wave, starfield, rain, fire, text reveal and scrolling text; deterministic seeds |
| Text effects | Reveal, typewriter, decrypt, scatter, wipes in 4 directions, rain, wave and burn, applied to any art or lettering, forwards or in reverse; set duration, FPS and seed |
| Media conversion | PNG, JPEG, GIF, WebP, BMP and PNM input; animated GIF timing; custom character ramps, aspect correction, transparency, inversion, gamma, contrast, dithering and edges; CLI batches and `https://` URLs |
| Video and camera | In the browser editor: any video the browser plays, sampled at 1 to 60 frames per second between chosen start and end times; a live text preview from the camera and timed recordings of 1 to 60 seconds |
| Browser editor | Pencil, eraser, line, rectangle, fill, typing, area select/move/copy/paste and reusable art stamps on the character grid, each usable by keyboard; optional directional ASCII mirrors; diagram mode for joined `+-|` boxes, labels and arrows; onion skin; select/copy text, timeline, frame-range timing/duplication/deletion/reordering, undo/redo, local file imports and opt-in local recovery |
| Export | Plain `.txt`, timed `.json` projects and standalone `.html` text players |
| Native playback | Terminal playback with cursor movement; saved artwork remains plain text |

All maintained application logic is Rust, including the browser's DOM controls and file handling. HTML and CSS describe the interface. A small JavaScript loader starts WebAssembly; standalone HTML exports include a small JavaScript text player so each file works on its own.

## Run the CLI

Download the archive for your platform from [Releases](https://github.com/EquityBot/ApollinaireASCII/releases/latest): Windows, macOS and Linux on x86-64 and ARM64, with `SHA256SUMS.txt`. Unpack it and put `apollinaire` on your `PATH`.

From v0.3.0, each archive has a signed build provenance attestation. To confirm an archive was built by this repository's release workflow from its release tag, run:

```sh
gh attestation verify apollinaire-v0.3.0-x86_64-unknown-linux-musl.tar.gz \
  --repo EquityBot/ApollinaireASCII \
  --signer-workflow EquityBot/ApollinaireASCII/.github/workflows/release.yml \
  --source-ref refs/tags/v0.3.0
```

To build from source instead, install [Rust](https://www.rust-lang.org/tools/install) 1.99 or later, then build the [crates.io release](https://crates.io/crates/apollinaire-ascii):

```sh
cargo install --locked apollinaire-ascii
```

For unreleased changes on `master`, use `cargo install --locked --git https://github.com/EquityBot/ApollinaireASCII`.

```sh
apollinaire art ghost
apollinaire list --tag bird
apollinaire text "RUST" --font block
apollinaire text "Hello there, friend" --font slant --wrap 40 --horizontal fitted
apollinaire animate ghost --output ghost.json
apollinaire text "HELLO" --font block | apollinaire effect decrypt - --output hello.json
apollinaire play ghost.json
apollinaire image photo.png --width 80 --dither --output photo.txt
apollinaire image a.png b.gif https://example.com/c.png --width 72 --output-dir ascii
apollinaire import ghost.json --format html --output ghost.html
```

The checked toolchain is pinned in `rust-toolchain.toml`. Existing output files are never overwritten. Static artwork defaults to text; animations default to JSON so frames and timings are retained. Use `--format text --frame 0` to deliberately select one animated frame.

## Use the library

The library has no browser or CLI dependency when default features are disabled:

```toml
[dependencies]
apollinaire-ascii = { version = "0.3", default-features = false }
```

```rust
use apollinaire_ascii::{Animation, Canvas, SceneOptions, animation};

fn main() -> Result<(), String> {
    let mut canvas = Canvas::new(24, 6)?;
    canvas.rectangle(0, 0, 24, 6, b'#')?;
    canvas.text(3, 2, "HELLO, FRIEND")?;
    let still = Animation::still(canvas.finish());
    println!("{}", still.frames()[0].frame.text());

    let moving = animation::generate("ghost", &SceneOptions::default())?;
    let round_trip = Animation::from_json(&moving.to_json())?;
    assert_eq!(moving, round_trip);
    Ok(())
}
```

`Frame` is an immutable rectangular ASCII grid. `Canvas` provides validated, clipped drawing, including flood fill and diagram boxes and arrows that join with `+`; `Animation` owns equal-size frames, durations and loop behaviour. See [the format and limits](docs/FORMAT.md) and the [API documentation](https://docs.rs/apollinaire-ascii).

## Build the browser editor

```sh
cargo build --locked --release --target wasm32-unknown-unknown --no-default-features --features web
cargo install --locked wasm-bindgen-cli --version 0.2.130
wasm-bindgen target/wasm32-unknown-unknown/release/apollinaire_ascii.wasm --target web --out-dir web/pkg --out-name apollinaire_ascii
```

Serve `web/` with a local HTTP server and open its URL. Opening `index.html` directly as a `file:` URL cannot load its WebAssembly module. GitHub Actions builds and deploys the same directory to GitHub Pages.

The editor processes files in your browser. It has no upload endpoint, account system, analytics or external font request. The camera is requested only when you choose Camera on, and every camera track stops when you turn it off, finish a recording or hide or leave the page. Camera frames are never sent over the network. The library and editor make no network requests; only the CLI downloads, and only an `https://` image URL you pass to `apollinaire image`.

Playback starts paused when reduced motion is requested and pauses when the page is hidden. Save JSON to keep all frames and their timings; text export saves the selected frame.

To reuse part of a drawing, select an area and choose **Save area as stamp** under the drawing options. Choose **Stamp art**, pick a saved stamp and drag to preview its placement. Release to place it, or use Space, the arrow keys and Space again; Escape cancels. The space toggle chooses between transparent spaces and overwriting the destination. Stamps stay in memory for this session only, up to 16 distinct entries and 1,000,000 cells in total. They survive frame and project changes but are not included in saved JSON; placed artwork is saved normally.

Flips move characters literally by default. Enable **Mirror directional characters when flipping** to reflect slash and bracket pairs horizontally, or slashes and `^`/`v` arrowheads vertically. Letters otherwise stay unchanged; leave the option off when `v` is text. The library exposes these choices through `Frame::mirror_horizontal` and `Frame::mirror_vertical`, alongside the existing literal flips.

Under **Edit characters and frames**, enter the first and last frame numbers and choose **Select frame range**. Both ends are included. Duplicate, delete and move operate on that range; **Apply duration to range** sets the duration of each selected frame. Duplication selects the copies and movement keeps their order. Each operation is one undo step, and undo/redo restores the active frame and selected range. At least one frame must remain. Manual seeking selects one frame; playback preserves the range. The library's `Animation` range methods use zero-based, end-exclusive Rust ranges.

Under **Local project recovery**, enable **Keep applied project edits in this browser** to save applied artwork and timings in IndexedDB after a short debounce. Recovery starts off on every page load; choose **Restore saved project** or **Discard saved recovery** when a saved project exists. Pausing retains the last saved project. Draft text, session stamps and undo history are excluded. Conflicting changes from another tab stop recovery and require an explicit choice. Discard removes the saved artwork and leaves only a revision marker to block stale writers. Conflicts, storage failures and a saved project found on load appear in the recovery panel and the main status line. A tab using recovery rechecks the saved record when you return to it. Browser clearing, eviction or a crash before a write completes can lose recovery data, so save JSON for a separate copy.

## Checks

```sh
cargo fmt --all --check
cargo clippy --locked --all-targets -- -D warnings
cargo test --locked
cargo build --locked --release
cargo clippy --locked --target wasm32-unknown-unknown --no-default-features --features web --lib --tests -- -D warnings
cargo test-editor
```

The integration tests cover the ASCII contract, whitespace, drawing, all bundled styles, scenes, effects and catalogue pieces, timing, round trips, malformed fonts, GIF preflight checks, video and camera frame sampling and timing, resource limits, script-safe HTML, CLI file handling and batch output, and URL limits against a local test server. `cargo test-editor` runs the editor tests in `src/web/tests.rs` in a headless browser. They embed the page markup and stylesheet, start the editor and drive it through pointer, keyboard, clipboard and form events: each drawing tool by pointer and by keyboard, diagram joins, onion skin, undo and redo, file opening, frame editing, effects and catalogue search, with every read of the artwork checked as printable ASCII. They need `wasm-bindgen-test-runner` from the pinned `wasm-bindgen-cli` 0.2.130 and `CHROMEDRIVER` set to a chromedriver matching an installed Chrome or Chromium. CI runs the native checks on Linux and Windows, runs the editor tests in headless Chrome on Linux and builds the browser module. See [validation](docs/VALIDATION.md) for the browser checks and known limits.

## When to choose something else

ApollinaireASCII keeps every output to printable ASCII and shares one Rust core between the CLI, library and browser editor. These projects suit other needs better; [the research notes](docs/RESEARCH.md) compare most of them:

- Colour, ANSI, Unicode or CP437 art: [durdraw](https://github.com/durdraw/durdraw), [chafa](https://github.com/hpjansson/chafa) or [libcaca](https://github.com/cacalabs/libcaca).
- Banner lettering alone: [FIGlet](http://www.figlet.org/), whose font format ApollinaireASCII reads.
- Diagrams alone: [ASCIIFlow](https://github.com/lewish/asciiflow) in a browser or [ASCII Draw](https://github.com/Nokse22/ascii-draw) on the GNOME desktop.
- Image conversion alone: [artem](https://github.com/FineFindus/artem) (Rust) or [ascii-image-converter](https://github.com/TheZoraiz/ascii-image-converter) (Go).
- Terminal animation and effects in Python: [asciimatics](https://github.com/peterbrittain/asciimatics) or [terminaltexteffects](https://github.com/ChrisBuilds/terminaltexteffects).
- Video playback in a terminal through FFmpeg: [tplay](https://github.com/maxcurzi/tplay).

## Foundation and licence

ApollinaireASCII retains the history of [orhnk/RASCII](https://github.com/orhnk/RASCII), starting at `8d33a3670e51298d530ab202090b212501310cdd`. Its MIT image-conversion approach is the foundation of the converter. The shared frame model, drawing, scenes, project format, editor and exporters are new Rust code.

[Research](docs/RESEARCH.md) records 1,164 discovered repositories across 27 multilingual searches, the shortlist and why this foundation was selected. Discovery metadata is not a source audit of every result. No Ghostty animation artwork was copied.

MIT for this repository's code. Preserve the [licence](LICENSE.md) and [third-party notices](THIRD_PARTY_NOTICES.md). Dependencies and bundled font data retain their own licences and notices. The banner's lettering and emblem are drawn from Cascadia Mono and Besley glyphs (SIL OFL 1.1).

---

<p align="center"><sub>Hello, friend. Keep it in characters.<br>Made by <a href="https://github.com/EquityBot">EquityBot</a>. <i>Ad gloriam eius, non meam.</i></sub></p>
