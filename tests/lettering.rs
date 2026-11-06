// Copyright 2026 Ryan Duguid
// SPDX-License-Identifier: MIT

//! Fixtures in `tests/fixtures/lettering` are reference output from the bundled fonts:
//! horizontal layout, wrap and right-to-left from figlet 2.2.5 (cmatsuoka/figlet `202a0a8`,
//! `figlet -l` with `-W`, `-k`, `-S` or `-s`; wrap width `w` is `-w w+1`), vertical layout from
//! figlet.js 1.12.0 `textSync`.
use apollinaire_ascii::lettering::{self, Alignment, Layout, Options};

fn fixture(name: &str) -> String {
    let path = format!(
        "{}/tests/fixtures/lettering/{name}.txt",
        env!("CARGO_MANIFEST_DIR")
    );
    let text = std::fs::read_to_string(path).unwrap();
    let rows: Vec<&str> = text.trim_end_matches('\n').split('\n').collect();
    let width = rows.iter().map(|r| r.len()).max().unwrap();
    rows.iter()
        .map(|r| format!("{r:width$}"))
        .collect::<Vec<_>>()
        .join("\n")
}

fn render(text: &str, font: &str, options: Options) -> String {
    lettering::render_with(text, font, None, &options)
        .unwrap()
        .text()
}

fn layout(horizontal: Layout, vertical: Layout) -> Options {
    Options {
        horizontal,
        vertical,
        ..Options::default()
    }
}

#[test]
fn horizontal_layouts_match_figlet() {
    let sample = "Ab_|/\\[]{}()<>";
    for (mode, name) in [
        (Layout::Full, "standard-full"),
        (Layout::Fitted, "standard-fitted"),
        (Layout::Smush, "standard-smush"),
        (Layout::Default, "standard-default"),
    ] {
        assert_eq!(
            render(sample, "standard", layout(mode, Layout::Full)),
            fixture(name),
            "{name}"
        );
    }
    // FIGlet's Block font fits by default; forced smushing without rules overlaps universally.
    let smushed = render(
        "Lo/\\X",
        "figlet-block",
        layout(Layout::Smush, Layout::Full),
    );
    assert_eq!(smushed, fixture("block-smush"));
    assert_ne!(
        smushed,
        render(
            "Lo/\\X",
            "figlet-block",
            layout(Layout::Default, Layout::Full)
        )
    );
    // Ivrit prints right to left.
    assert_eq!(
        render("Ab c", "ivrit", Options::default()),
        fixture("ivrit-default")
    );
}

#[test]
fn vertical_layouts_match_figlet_js() {
    let text = "|_|\n-_-\n[/]";
    for (mode, name) in [
        (Layout::Full, "standard-vertical-full"),
        (Layout::Fitted, "standard-vertical-fitted"),
        (Layout::Smush, "standard-vertical-smush"),
        // Standard's header selects vertical smushing with all 5 rules.
        (Layout::Default, "standard-vertical-smush"),
    ] {
        assert_eq!(
            render(text, "standard", layout(Layout::Full, mode)),
            fixture(name),
            "{name}"
        );
    }
    // Banner has no vertical rules, so smushing is universal.
    assert_eq!(
        render("Hi\nyo", "banner", layout(Layout::Full, Layout::Smush)),
        fixture("banner-vertical-smush")
    );
}

#[test]
fn word_wrap_matches_figlet_and_is_separate_from_canvas_width() {
    let wrap = |columns| Options {
        wrap: Some(columns),
        vertical: Layout::Full,
        ..Options::default()
    };
    assert_eq!(
        render("Present day, present time", "standard", wrap(30)),
        fixture("standard-wrap-30")
    );
    // A word wider than the wrap width breaks between letters.
    assert_eq!(render("Overlong", "big", wrap(12)), fixture("big-wrap-12"));
    let canvas = Options {
        alignment: Alignment::Center,
        width: Some(80),
        ..Options::default()
    };
    let centred = lettering::render_with("Hello there", "standard", None, &canvas).unwrap();
    assert_eq!((centred.width(), centred.height()), (80, 6));
    let narrow = Options {
        width: Some(30),
        ..Options::default()
    };
    assert!(lettering::render_with("Hello there", "standard", None, &narrow).is_err());
    assert!(lettering::render_with("W", "big", None, &wrap(3)).is_err());
    assert!(lettering::render_with("W", "big", None, &wrap(0)).is_err());
}

#[test]
fn multiline_alignment_centres_each_line() {
    let options = Options {
        alignment: Alignment::Center,
        vertical: Layout::Full,
        ..Options::default()
    };
    let frame = lettering::render_with("I\nIII", "mnemonic", None, &options).unwrap();
    assert_eq!(frame.text(), " I \nIII");
    // An empty line is a blank line of lettering, as in figlet.
    let blank = lettering::render_with("\n", "standard", None, &Options::default()).unwrap();
    assert_eq!((blank.width(), blank.text().trim()), (1, ""));
}

fn toilet_font() -> String {
    let mut font =
        String::from("tlf2a$ 1 1 4 -1 1 0 0 0\nComments may use UTF-8: \u{201c}Letter\u{201d}\n");
    for byte in 32..=126u8 {
        let c = char::from(byte);
        let marker = if c == '@' { '#' } else { '@' };
        font.push_str(&format!("{c}{marker}\n"));
    }
    font
}

#[test]
fn toilet_fonts_import_only_when_every_glyph_is_printable_ascii() {
    let font = toilet_font();
    let frame = lettering::render("Hi $", "custom", Some(&font), Alignment::Left, None).unwrap();
    assert_eq!(frame.text(), "Hi  ", "the hardblank renders as a space");
    let mut extended = font.clone();
    extended.push_str(&"xx@\n".repeat(7));
    extended.push_str("0x2500 BOX DRAWINGS LIGHT HORIZONTAL \u{2500}\n--@\n");
    assert!(lettering::render("Hi", "custom", Some(&extended), Alignment::Left, None).is_ok());
    let refuse = |font: &str, needle: &str| {
        let error =
            lettering::render("Hi", "custom", Some(font), Alignment::Left, None).unwrap_err();
        assert!(error.contains(needle), "{error}");
    };
    refuse(
        &extended.replace("--@\n", "\u{2500}@\n"),
        "0x2500 row 1 has byte 0xe2",
    );
    refuse(&extended.replacen("xx@\n", "\u{c4}@\n", 1), "196 row 1");
    refuse(&font.replace("A@\n", "\u{c1}@\n"), "'A' row 1");
    refuse(&format!("{font}x@\n"), "missing rows");
    refuse(&font.replace("A@\n", "A@\t\n"), "'A' row 1 has byte 0x09");
    refuse(
        &format!("{extended}not a tag\n"),
        "neither a glyph row nor a code tag",
    );
    // Fonts are checked without rendering, so glyphs too wide for a sample word still load.
    let mut wide = font.clone();
    for c in "ACIS".chars() {
        wide = wide.replace(&format!("{c}@\n"), &format!("{}@\n", "x".repeat(110)));
    }
    lettering::validate_font(&wide).unwrap();
    assert!(lettering::render("ASCII", "custom", Some(&wide), Alignment::Left, None).is_err());
    refuse("PK\u{3}\u{4}tlf2a$ 1 1 4 -1 0", "Compressed fonts");
    refuse("tlf2\u{2500} 1 1 4 -1 0", "header");
}
