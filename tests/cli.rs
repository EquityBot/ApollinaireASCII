// Copyright 2026 Ryan Duguid
// SPDX-License-Identifier: MIT

#![cfg(feature = "cli")]
mod common;
use std::io::Write;
use std::process::Command;
fn cli(args: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_apollinaire"))
        .args(args)
        .output()
        .unwrap()
}
#[test]
fn native_commands_emit_real_text_and_validate_input() {
    let art = cli(&["art", "ghost"]);
    assert!(art.status.success());
    assert!(
        art.stdout
            .iter()
            .all(|b| *b == b'\n' || (32..=126).contains(b))
    );
    assert!(!art.stdout.contains(&27));
    let text = cli(&["text", "Hello", "--font", "small"]);
    assert!(text.status.success());
    let unicode = cli(&["text", "👻"]);
    assert!(!unicode.status.success());
    assert!(unicode.stdout.is_empty());
    let animation = cli(&[
        "animate", "wave", "--frames", "3", "--width", "20", "--height", "8",
    ]);
    assert!(animation.status.success());
    let project =
        apollinaire_ascii::Animation::from_json(std::str::from_utf8(&animation.stdout).unwrap())
            .unwrap();
    assert_eq!(project.frames().len(), 3);
    let ambiguous = cli(&["animate", "ghost", "--format", "text"]);
    assert!(!ambiguous.status.success());
    let selected = cli(&["animate", "ghost", "--format", "text", "--frame", "1"]);
    assert!(selected.status.success());
}
#[test]
fn output_files_are_never_overwritten_and_playback_refuses_redirection() {
    let path = std::env::temp_dir().join(format!("apollinaire-cli-{}.txt", std::process::id()));
    std::fs::write(&path, "keep this").unwrap();
    let output = cli(&["art", "cat", "--output", path.to_str().unwrap()]);
    assert!(!output.status.success());
    assert_eq!(std::fs::read_to_string(&path).unwrap(), "keep this");
    let play = cli(&["play", path.to_str().unwrap()]);
    assert!(!play.status.success());
    assert!(play.stdout.is_empty());
    std::fs::remove_file(&path).unwrap();
}

#[test]
fn stdin_ascii_can_start_with_an_opening_brace() {
    let mut child = Command::new(env!("CARGO_BIN_EXE_apollinaire"))
        .args(["import", "-"])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .unwrap();
    child.stdin.take().unwrap().write_all(b"{ a }").unwrap();
    let output = child.wait_with_output().unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(output.stdout, b"{ a }\n");
}

fn scratch(name: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("apollinaire-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}
fn text(bytes: &[u8]) -> &str {
    std::str::from_utf8(bytes).unwrap()
}
#[test]
fn list_searches_tags_and_effects_animate_files_and_stdin() {
    let birds = cli(&["list", "--tag", "bird"]);
    assert!(birds.status.success());
    let names: Vec<_> = text(&birds.stdout)
        .lines()
        .map(|l| l.split(' ').next().unwrap())
        .collect();
    assert!(names.contains(&"duck") && names.contains(&"owl") && !names.contains(&"cat"));
    assert!(!cli(&["list", "--tag", "no-such-tag"]).status.success());
    assert!(text(&cli(&["list"]).stdout).contains("Effects: reveal"));

    let dir = scratch("effect");
    let art = dir.join("fox.txt");
    assert!(
        cli(&["art", "fox", "--output", art.to_str().unwrap()])
            .status
            .success()
    );
    let output = cli(&[
        "effect",
        "scatter",
        art.to_str().unwrap(),
        "--duration",
        "500",
        "--fps",
        "10",
    ]);
    assert!(output.status.success(), "{}", text(&output.stderr));
    let project = apollinaire_ascii::Animation::from_json(text(&output.stdout)).unwrap();
    assert_eq!(project.frames().len(), 5);
    let fox = apollinaire_ascii::art::template("fox").unwrap();
    assert_eq!(project.frames()[4].frame, fox);
    let leaving = cli(&[
        "effect",
        "scatter",
        "--art",
        "fox",
        "--reverse",
        "--duration",
        "500",
        "--fps",
        "10",
    ]);
    assert!(leaving.status.success(), "{}", text(&leaving.stderr));
    let leaving = apollinaire_ascii::Animation::from_json(text(&leaving.stdout)).unwrap();
    assert!(
        leaving.frames().iter().map(|f| &f.frame).eq(project
            .frames()
            .iter()
            .rev()
            .map(|f| &f.frame))
    );
    assert!(!cli(&["effect", "scatter"]).status.success());
    assert!(
        !cli(&["effect", "scatter", art.to_str().unwrap(), "--art", "fox"])
            .status
            .success()
    );
    assert!(
        !cli(&["effect", "scatter", "--art", "unicorn"])
            .status
            .success()
    );
    assert!(
        !cli(&["effect", "sparkle", art.to_str().unwrap()])
            .status
            .success()
    );

    let mut child = Command::new(env!("CARGO_BIN_EXE_apollinaire"))
        .args(["effect", "decrypt", "-", "--format", "text", "--frame", "0"])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(b"HELLO\nFRIEND\n")
        .unwrap();
    let output = child.wait_with_output().unwrap();
    assert!(output.status.success());
    assert!(
        output
            .stdout
            .iter()
            .all(|b| *b == b'\n' || (32..=126).contains(b))
    );
    assert_ne!(output.stdout, b"HELLO \nFRIEND\n");
    std::fs::remove_dir_all(dir).unwrap();
}
#[test]
fn image_edge_glyphs_draw_direction_and_edges_alone_keep_the_ramp() {
    let dir = scratch("edge-glyphs");
    let path = dir.join("step.png");
    image::RgbaImage::from_fn(4, 1, |x, _| {
        image::Rgba(if x < 2 { [0, 0, 0, 255] } else { [255; 4] })
    })
    .save(&path)
    .unwrap();
    let run = |flag: &str| {
        let image = path.to_str().unwrap();
        let output = cli(&["image", image, "--width", "4", "--height", "1", flag]);
        assert!(output.status.success());
        text(&output.stdout).to_string()
    };
    assert_eq!(run("--edges"), " @@ \n");
    assert_eq!(run("--edge-glyphs"), " || \n");
    std::fs::remove_dir_all(dir).unwrap();
}
#[test]
fn image_batches_create_one_file_per_input_and_never_overwrite() {
    let dir = scratch("batch");
    let picture = |name: &str| {
        let path = dir.join(name);
        image::DynamicImage::new_rgba8(16, 8).save(&path).unwrap();
        path.to_str().unwrap().to_string()
    };
    let (a, b) = (picture("a.png"), picture("b.png"));
    std::fs::create_dir(dir.join("sub")).unwrap();
    let nested = picture("sub/a.png");
    let bad = dir.join("notes.png");
    std::fs::write(&bad, "not an image").unwrap();
    let bad = bad.to_str().unwrap();
    let out = dir.join("out");
    let out_arg = out.to_str().unwrap();

    let first = cli(&["image", &a, &b, "--width", "8", "--output-dir", out_arg]);
    assert!(first.status.success(), "{}", text(&first.stderr));
    let a_text = std::fs::read(out.join("a.png.txt")).unwrap();
    assert!(a_text.iter().all(|b| *b == b'\n' || (32..=126).contains(b)));
    assert!(out.join("b.png.txt").exists());
    assert_eq!(text(&first.stdout).lines().count(), 2);

    std::fs::write(out.join("b.png.txt"), "keep this").unwrap();
    let again = cli(&["image", &a, &b, "--width", "8", "--output-dir", out_arg]);
    assert!(!again.status.success());
    assert!(text(&again.stderr).contains("2 of 2 inputs failed"));
    assert_eq!(std::fs::read(out.join("a.png.txt")).unwrap(), a_text);
    assert_eq!(
        std::fs::read_to_string(out.join("b.png.txt")).unwrap(),
        "keep this"
    );

    let partial = dir.join("partial");
    let mixed = cli(&[
        "image",
        bad,
        &a,
        "--width",
        "8",
        "--output-dir",
        partial.to_str().unwrap(),
    ]);
    assert!(!mixed.status.success());
    assert!(text(&mixed.stderr).contains("notes.png"));
    assert!(partial.join("a.png.txt").exists() && !partial.join("notes.png.txt").exists());

    let clash = dir.join("clash");
    let same = cli(&[
        "image",
        &a,
        &nested,
        "--output-dir",
        clash.to_str().unwrap(),
    ]);
    assert!(!same.status.success());
    assert!(!clash.exists());
    let upper = picture("A.PNG");
    let case = cli(&["image", &a, &upper, "--output-dir", clash.to_str().unwrap()]);
    assert!(text(&case.stderr).contains("Several inputs would be named"));
    assert!(!clash.exists());
    let gif = dir.join("a.gif");
    let frame = common::GifFrame::full(16, 8, vec![0; 128], 0);
    std::fs::write(&gif, common::gif(16, 8, Some(&[[0, 0, 0]]), &[frame])).unwrap();
    let gif = gif.to_str().unwrap().to_string();
    let kinds = dir.join("kinds");
    let both = cli(&[
        "image",
        &a,
        &gif,
        "--width",
        "8",
        "--output-dir",
        kinds.to_str().unwrap(),
    ]);
    assert!(both.status.success(), "{}", text(&both.stderr));
    assert!(kinds.join("a.png.txt").exists() && kinds.join("a.gif.txt").exists());
    assert!(!cli(&["image", &a, &b]).status.success());
    assert!(
        !cli(&["image", "-", &a, "--output-dir", out_arg])
            .status
            .success()
    );

    let plain = cli(&["image", "http://127.0.0.1:9/a.png"]);
    assert!(!plain.status.success());
    assert!(text(&plain.stderr).contains("Only https:// URLs are accepted"));
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn text_accepts_layout_wrap_and_ascii_toilet_fonts_only() {
    let wrapped = cli(&[
        "text",
        "Present day",
        "--wrap",
        "40",
        "--horizontal",
        "fitted",
        "--vertical",
        "smush",
    ]);
    assert!(wrapped.status.success());
    let rows: Vec<&[u8]> = wrapped
        .stdout
        .trim_ascii_end()
        .split(|b| *b == b'\n')
        .collect();
    assert!(rows.len() > 6 && rows.iter().all(|r| r.len() <= 40));
    let mut font = String::from("tlf2a$ 1 1 4 -1 0\n");
    for byte in 32..=126u8 {
        let marker = if byte == b'@' { '#' } else { '@' };
        font.push_str(&format!("{}{marker}\n", char::from(byte)));
    }
    let path = std::env::temp_dir().join(format!("apollinaire-cli-{}.tlf", std::process::id()));
    std::fs::write(&path, &font).unwrap();
    let ok = cli(&["text", "Hi", "--font-file", path.to_str().unwrap()]);
    assert_eq!(ok.stdout, b"Hi\n");
    std::fs::write(&path, font.replace("H@\n", "\u{2588}@\n")).unwrap();
    let refused = cli(&["text", "Hi", "--font-file", path.to_str().unwrap()]);
    assert!(!refused.status.success() && refused.stdout.is_empty());
    assert!(String::from_utf8_lossy(&refused.stderr).contains("glyph 'H' row 1 has byte 0xe2"));
    std::fs::remove_file(&path).unwrap();
}
