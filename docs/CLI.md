# CLI guide

`apollinaire --help` lists commands. Each command also accepts `--help`.

```sh
apollinaire list
apollinaire list --tag sea
apollinaire art cat --border --output cat.txt
apollinaire effect typewriter cat.txt --duration 3000 --fps 30 --output cat.json
apollinaire art rocket | apollinaire effect burn - --seed 7 --format html --output rocket.html
apollinaire effect wave movie.json --source-frame 4 --output wave.json
apollinaire effect scatter --art ghost --reverse --output goodbye.json
apollinaire text "Hello" --font small --align center --width 72
apollinaire text "RUST" --font-file my-font.flf
apollinaire text "Hello there, friend" --font smslant --wrap 30 --align center
apollinaire text "$(printf 'TOP\nLOW')" --horizontal smush --vertical fitted
apollinaire animate donut --width 72 --height 24 --frames 120 --fps 30 --output donut.json
apollinaire animate scroll --text "HELLO, FRIEND" --seed 42 --output scroll.html
apollinaire image photo.jpg --width 100 --ramp fine --gamma 1.2 --contrast 1.3 --dither
apollinaire image movie.gif --width 72 --output movie.json
apollinaire image logo.png --width 72 --edges --invert --background 255
apollinaire image logo.png --width 72 --edge-glyphs
apollinaire image shots/*.png intro.gif --width 72 --output-dir ascii
apollinaire image https://example.com/photo.jpg --width 80 --output photo.txt
apollinaire import art.txt --pad --format json --output art.json
apollinaire import movie.json --format html --output movie.html
apollinaire import movie.json --format text --frame 0 --output first.txt
apollinaire compose background.txt cat.txt --x 4 --y 2 --transparent-spaces
apollinaire transform movie.json horizontal --output flipped.json
apollinaire transform art.txt crop --x 2 --y 1 --width 20 --height 10
apollinaire transform art.txt border
apollinaire check movie.json
apollinaire play movie.json --loops 3
```

`--output` creates a new file. An existing file causes an error, even when its contents are identical. Without it, exports go to stdout. Errors go to stderr and return a nonzero exit status. `check` reports validated dimensions and timing on stdout.

`--format text|json|html` overrides extension/default inference. `.json` and `.html` output extensions select those formats; otherwise a still defaults to text and an animation to JSON. Animated text export requires an explicit zero-based `--frame`. `--frame` is valid only with text export.

Use `-` as the input path for stdin. `.json` input is always parsed as a project and `.txt` as text. Ambiguous input, including stdin, is treated as JSON when it begins with `{` and is syntactically JSON; otherwise it is text. Give literal artwork that is also valid JSON a `.txt` filename to force text interpretation. CRLF is normalised to LF for text imports. `--pad` explicitly pads ragged rows; the browser pads text imports automatically.

`image` accepts a named ramp (`standard`, `fine`, `simple`, `binary`) or a literal sequence ordered from light to dense. Quote literal ramps containing spaces or shell characters. `--cell-aspect` defaults to `0.5`; use `--height` to set rows explicitly. The background is a greyscale value from 0 to 255 for compositing transparency. Gamma and contrast accept 0.1 to 5.0. `--edges` maps edge strength through the ramp. `--edge-glyphs` does too, but draws strong edges (at least a quarter of a full black-to-white step) as `|`, `/`, `-` or `\` along their direction. Animated GIF input retains each frame's delay, normalising zero delay to 100 ms and applying the format's 10 to 60,000 ms limits.

`list --tag TEXT` prints each catalogue piece whose name, category or tag contains the text, ignoring case, with its category and tags. It fails when nothing matches.

`effect NAME INPUT` animates one frame of a `.txt` or `.json` input (or `-` for stdin), or a catalogue piece with `--art NAME` instead of an input: `reveal`, `typewriter`, `decrypt`, `scatter`, `wipe-left`, `wipe-right`, `wipe-up`, `wipe-down`, `rain`, `wave` or `burn`. Spaces are background; the other characters move, appear or burn. The frame count is `--duration` (100 to 60,000 ms, default 2,000) times `--fps` (1 to 60, default 20) divided by 1,000, rounded up and at least 2, within the 600-frame and 8,000,000-cell project limits. The frames share the duration exactly. The same `--seed` always gives the same frames. Every effect except `burn` ends on the unchanged source; `burn` ends blank. `--reverse` plays the frames backwards, so the art leaves instead of arriving. `--source-frame` chooses the frame of a JSON project, counting from zero. Ragged text rows are padded.

`image` accepts several inputs with `--output-dir DIR`. It creates the directory if needed and writes one new file per input, named after the input's file name plus the output extension: `.txt` for a still and `.json` for an animated GIF unless `--format` says otherwise, so `clip.gif` becomes `clip.gif.json`. Inputs whose names would collide, ignoring letter case, are refused before anything is written. Existing files are never replaced. A failed input is reported on stderr and the others are still converted; the exit status is nonzero if any input failed.

An `image` input starting with `https://` is downloaded: at most 16 MiB, within 30 seconds including at most 5 redirects, all over HTTPS. The response must have an `image/` content type, otherwise the command fails with the type it received. Plain `http://` is refused. The CLI is the only part that uses the network; the library and browser editor never do.

`text` uses figlet's layout rules. `--horizontal` sets how letters join and `--vertical` how lines of lettering join: `default` (the font's own setting), `full` (no overlap), `fitted` (touching) or `smush` (merged by the font's smushing rules, or universally when the font has none). `--wrap N` breaks lines at spaces, or inside a word that is too long, so none exceeds N columns; it matches `figlet -w N+1`, because figlet reserves the terminal's last column. `--width` sets the canvas width that `--align` uses and does not wrap; each line of lettering is aligned separately, as `figlet -c` and `-r` do. `apollinaire list` names the bundled fonts. `--font-file` accepts a plain FIGlet `.flf` or TOIlet `.tlf` file up to 1 MiB whose every glyph is drawn in printable ASCII; files with other bytes in any glyph, and compressed fonts, are refused with the glyph and byte named.

`play` requires a terminal and refuses redirected stdout. It uses ANSI clear/cursor commands solely to update terminal text. It does not put those codes in the project or any text export. It plays the requested number of cycles; the project's loop setting controls the browser and standalone player. Unsupported media and malformed input fail visibly.
