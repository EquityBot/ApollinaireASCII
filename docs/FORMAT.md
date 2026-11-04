# Text and project format

Artwork cells are printable ASCII bytes 32 through 126, inclusive. LF separates rows in serialised text. Tabs, CR, NUL, DEL, escape codes, accented characters, emoji and Unicode block characters are rejected. Text imports may normalise CRLF before calling the core parser. Trailing spaces are significant; no final LF is stored in `Frame::text()`.

Frames are nonempty rectangles. `Frame::from_text` requires equal-width rows. `Frame::from_text_padded` explicitly pads shorter rows on the right. Both permit one final LF. A blank canvas is a rectangle of spaces rather than an empty frame.

A version 1 JSON project looks like this:

```json
{"format":"asciiforge","version":1,"width":3,"height":1,"loop":true,"frames":[{"text":"o o","duration_ms":100},{"text":"- -","duration_ms":150}]}
```

All fields are required and no additional or duplicate fields are accepted. JSON property order is immaterial. Frame dimensions must match the project. Import validates typed fields and budgets during deserialisation rather than allocating an arbitrary JSON value tree. The immutable frame type prevents invalid artwork from entering a valid project through the public API.

| Limit | Maximum |
| --- | --- |
| Canvas | 512 columns by 256 rows |
| Frames | 600 |
| Total project cells | 8,000,000 |
| Frame duration | 10 to 60,000 ms |
| Project duration | One hour |
| Effect duration | 100 to 60,000 ms at 1 to 60 FPS |
| File input | 16 MiB |
| CLI URL input | `https://` only, 16 MiB, 30 seconds, 5 redirects, `image/` content type |
| Media dimensions | 4,096 per side, 16,777,216 pixels |
| Image decoder allocation limit | 128 MiB |
| GIF logical-screen pixels across frames | 64,000,000 |
| Custom font file | 1 MiB |
| Video and camera sampling | 1 to 60 frames per second; camera recordings of 1 to 60 seconds |
| Video and camera pixels read per frame | 2 per output column across, aspect preserved |
| Custom font height / row length | 32 rows / 256 bytes |
| Browser undo history | 100 states and 16,000,000 cells |
| Browser art stamp library | 16 distinct stamps and 1,000,000 cells in total; session memory only |
| Browser recovery project JSON | 16 MiB encoded UTF-8, plus a bounded revision header |

Browser recovery is separate from project JSON v1. Its opt-in IndexedDB slot contains a random revision line followed by the unchanged project JSON. Restores use the existing bounded project parser. A discard replaces the artwork with a revision-only marker; automatic writers compare the exact observed record within a read/write transaction, so stale writers cannot overwrite a later save or discard. Storage availability and quota remain browser controlled. Recovery excludes unapplied text, stamps and history.

Frame-range operations preserve project limits and leave the project unchanged on failure. Browser ranges show 1-based inclusive frame numbers; the Rust methods accept nonempty, zero-based, end-exclusive ranges. `move_frames` takes the range's destination index in the resulting project. History includes the active frame and chosen range alongside each project snapshot.

Video and camera input in the browser is checked against the frame, cell and media dimension limits before any frame is read. Video files are not read into memory by the editor; the browser decodes them from a local object URL, and each sampled frame is drawn into an offscreen canvas only to read its pixels for the converter. Frame durations come from the sample times, rounded to milliseconds so they sum to the clip or recording length.

GIF headers, image rectangles, frame count, aggregate decoded screen pixels and output cell budget are inspected before frame decoding. Image rectangles must be nonzero and fit inside the logical screen. Decoder limits complement these checks; they are not a process-wide memory cap. The browser retains the current project, bounded history and temporary decode data, so peak memory can exceed the stored character count.

Plain FIGlet `.flf` and TOIlet `.tlf` fonts must define all 95 printable ASCII glyphs, with rows of equal width within each glyph and one or two end markers. Markers may differ between glyphs. Every glyph in the file, including the 7 German and any code-tagged glyphs, must be drawn in printable ASCII once the hardblank becomes a space; otherwise the font is refused with the glyph, row and byte named. Comments may contain any UTF-8. Compressed font archives are not accepted. Rendering that exceeds canvas limits, or a glyph wider than the wrap width, is refused.

Horizontal layout and word wrap follow figlet 2.2.5. Vertical layout follows the FIGfont 2 specification as figlet.js implements it, with one difference: figlet.js smushes lines vertically by default whenever a font lists vertical smushing rules, whereas the specification, and ApollinaireASCII, apply those rules only when the font's header or the user selects vertical smushing. Bundled lean, mini, script, shadow and figlet-block therefore stack lines at full height by default.

JSON stores characters and timing, not pixels. HTML exports embed the JSON safely and put each frame into a `<pre>` using `textContent`; they contain a small standalone JavaScript player. Neither format includes raster or vector artwork. The player starts paused, supports stepping and pauses on a hidden page or a change to reduced motion.
