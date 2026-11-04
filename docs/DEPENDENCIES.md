# Dependency scan and reachability

Checked on 1 November 2026 against the committed Cargo.lock. Scan alerts describe package versions; reachability conclusions below concern ApollinaireASCII's current calls and do not declare those packages free of vulnerabilities.

## Fixed reachable decoder alert

Aikido reported `AIKIDO-2026-10928` in `weezl` 0.1.12. GIF input reaches the LZW decoder, so file limits alone do not remove the exposure. The latest `gif` release, 0.14.2, still uses `weezl` 0.1; the [upstream fix](https://github.com/image-rs/image-gif/commit/e60e4dd7708cc50f9371185cefa04273f410f116) moving it to 0.2.1 is unreleased. A `[patch]` pin protects only the workspace that declares it, not applications that depend on this library, so ApollinaireASCII decodes GIF frames itself in `src/gif.rs` with `weezl` 0.2.1 and builds `image` without its GIF feature. Cargo.lock resolves `weezl` 0.2.1 and no `gif` crate, for this repository and for library users. [Upstream changes](https://github.com/image-rs/weezl/blob/v0.2.0/Changes.md#version-020) describe malformed-stream and progress fixes.

The decoder reproduces the frames `image` 0.25.10 produced with the patched `gif` crate. A comparison covered 11 real GIFs, 2 hand-built LZW edge cases, 3,000 generated animations and 63,231 corrupted or truncated variants; 1,946 variants were skipped because their screens exceed the 4096-pixel preflight limit. Every input both decoders accepted gave identical pixels and delays, and neither panicked. They differ only in accepting malformed files:

- Frames outside the logical screen, and files with no frames: the preflight check refuses these, as it did before.
- 6 corrupt interlaced frames had an invalid LZW code after the last pixel. `gif` reads interlaced data a row at a time and reached that code; this decoder stops once every pixel is decoded and accepts them.
- 1 file's image data began with an empty sub-block. `gif` read the next byte as more image data; this decoder reports the frame as truncated.

## Remaining package alerts

| Locked package | Aikido alerts | Current application reachability |
| --- | --- | --- |
| `moxcms` 0.8.1, through `image` 0.25.10 | `AIKIDO-2026-175889` | Its affected ICC byte parser is not called. Conversion decodes pixels without interpreting embedded ICC profiles. |

`DynamicImage::from_decoder` in the locked image crate decodes pixels and assumes sRGB. It does not pass embedded ICC bytes to `moxcms::ColorProfile::new_from_slice` or `new_from_slice_with_options`. Image's profile construction uses built-in colour metadata. This supports non-reachability of the specific ICC LUT parsing alert; it is not a conclusion about every colour-processing defect. See the [moxcms 0.9.0 fixes](https://github.com/awxkee/moxcms/releases/tag/0.9.0).

At this check, the latest stable `image` release still requires the older dependency range. The alert remains visible and is not ignored or snoozed in the scanner.

ApollinaireASCII replaced `figlet-rs` 1.0.0 with its own FIGlet renderer on 1 November 2026, which removed `zip` 0.6.6 and its 4 alerts (`CVE-2026-115905`, `CVE-2026-115906`, `AIKIDO-2026-476773`, `AIKIDO-2026-306911`) from Cargo.lock. Aikido has not been rerun since that change. Fonts are read as bounded plain text; compressed font archives are refused. Reassess these conclusions when adding archive APIs, ICC colour management or changing the dependency graph. Upgrade through upstream releases when compatible fixes become available.

## CLI HTTPS client

`ureq` 3.5.0 downloads `https://` image URLs for `apollinaire image`. It is optional and enabled only by the `cli` feature, with its gzip support off and the `rustls` TLS backend, which brings `ring` and the `webpki-roots` certificate bundle. The library built with `default-features = false` and the browser module do not compile it. Responses are limited to 16 MiB, 30 seconds, 5 redirects, HTTPS throughout and an `image/` content type before the bytes reach the same image decoder and limits as local files. On 1 November 2026 an Aikido dependency scan of `Cargo.lock` at 7728d14 covered all 107 locked packages, including these, and reported only the 5 alerts above. `cargo audit` 0.22.2 with the RustSec database of that day reported only `RUSTSEC-2026-0367`, `RUSTSEC-2026-0368` and `RUSTSEC-2026-0369`, all in `zip` 0.6.6 archive extraction. All 3 left with `zip` when `figlet-rs` was replaced; `cargo audit` has not been rerun since.

## Source scan

On 3 November 2026, Aikido rescanned Cargo.toml and Cargo.lock after adding browser API bindings for local recovery. It reported only the existing `moxcms` 0.8.1 alert, `AIKIDO-2026-175889`; dependency versions and the lockfile were unchanged. The source scan of 8 files covering recovery, frame ranges and their integration reported zero issues and zero skipped paths. No alert was ignored or snoozed.

Aikido scanned the 11 Rust source files, browser HTML/CSS and 2 GitHub configuration files: 15 files, zero issues and zero skipped paths. This automated result complements the contract tests and independent code reviews; it is not an assurance that every possible defect was found.
