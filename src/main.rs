mod fetch;

use apollinaire_ascii::{
    Animation, Canvas, EffectOptions, Frame, MAX_INPUT_BYTES, RenderOptions, Result, SceneOptions,
    TimedFrame, animation, art, effects, export, lettering,
};
use clap::{Parser, Subcommand, ValueEnum};
use std::{
    fs::File,
    io::{self, IsTerminal, Read, Write},
    path::{Path, PathBuf},
    time::Duration,
};

#[derive(Parser)]
#[command(
    name = "apollinaire",
    version,
    about = "Static art, lettering and animation as real ASCII text"
)]
struct Args {
    #[command(subcommand)]
    command: Command,
    #[command(flatten)]
    export: Export,
}
#[derive(clap::Args)]
struct Export {
    /// Create this file. Existing files are never overwritten.
    #[arg(short, long, global = true)]
    output: Option<PathBuf>,
    /// Defaults to text for still art and JSON for animation.
    #[arg(long, global = true, value_enum)]
    format: Option<Format>,
    /// Export one zero-based frame when selecting text from an animation.
    #[arg(long, global = true)]
    frame: Option<usize>,
}
#[derive(Clone, Copy, ValueEnum)]
enum Format {
    Text,
    Json,
    Html,
}
#[derive(Clone, Copy, ValueEnum)]
enum Align {
    Left,
    Center,
    Right,
}
#[derive(Clone, Copy, ValueEnum)]
enum Layout {
    Default,
    Full,
    Fitted,
    Smush,
}
#[derive(Clone, Copy, ValueEnum)]
enum Transform {
    Horizontal,
    Vertical,
    Border,
    Crop,
}
#[derive(Subcommand)]
enum Command {
    /// List artwork, fonts, scenes, effects and conversion ramps.
    List {
        /// Show only artwork whose name, category or tag contains this text.
        #[arg(long)]
        tag: Option<String>,
    },
    /// Original static ASCII templates.
    Art {
        name: String,
        #[arg(long)]
        border: bool,
    },
    /// ASCII block letters, including punctuation and multiline text.
    Text {
        text: String,
        #[arg(long, default_value = "standard")]
        font: String,
        /// Plain FIGlet .flf or TOIlet .tlf font drawn in printable ASCII.
        #[arg(long)]
        font_file: Option<PathBuf>,
        #[arg(long, value_enum, default_value = "left")]
        align: Align,
        /// Canvas width for alignment.
        #[arg(long)]
        width: Option<usize>,
        /// Break lines, at spaces where possible, to at most this many columns.
        #[arg(long)]
        wrap: Option<usize>,
        /// How letters join: the font's default, full width, fitted (kerning) or smushed.
        #[arg(long, value_enum, default_value = "default")]
        horizontal: Layout,
        /// How lines join: the font's default, full height, fitted or smushed.
        #[arg(long, value_enum, default_value = "default")]
        vertical: Layout,
    },
    /// Convert images or animated GIFs to text, from files or https:// URLs.
    Image {
        /// Image files, `-` for stdin, or https:// URLs. Several inputs need --output-dir.
        #[arg(required = true)]
        inputs: Vec<PathBuf>,
        /// Create one new file per input in this directory, named after the input.
        #[arg(long)]
        output_dir: Option<PathBuf>,
        #[arg(short, long, default_value_t = 80)]
        width: usize,
        #[arg(long)]
        height: Option<usize>,
        #[arg(long, default_value = "standard")]
        ramp: String,
        #[arg(long)]
        invert: bool,
        #[arg(long)]
        dither: bool,
        #[arg(long)]
        edges: bool,
        /// Trace edges, drawing strong ones as | / - \ along their direction.
        #[arg(long)]
        edge_glyphs: bool,
        #[arg(long, default_value_t = 1.0)]
        gamma: f64,
        #[arg(long, default_value_t = 1.0)]
        contrast: f64,
        #[arg(long, default_value_t = 0.5)]
        cell_aspect: f64,
        #[arg(long, default_value_t = 0)]
        background: u8,
    },
    /// Deterministic procedural ASCII scenes, exported as timed JSON.
    Animate {
        #[arg(default_value = "ghost")]
        scene: String,
        #[arg(long, default_value_t = 72)]
        width: usize,
        #[arg(long, default_value_t = 24)]
        height: usize,
        #[arg(long, default_value_t = 80)]
        frames: usize,
        #[arg(long, default_value_t = 20)]
        fps: u32,
        #[arg(long, default_value_t = 42)]
        seed: u32,
        #[arg(long, default_value = "HELLO, FRIEND")]
        text: String,
    },
    /// Animate text art or lettering with a deterministic effect, exported as timed JSON.
    Effect {
        #[arg(value_parser = clap::builder::PossibleValuesParser::new(effects::EFFECTS))]
        effect: String,
        /// A .txt or .json file, or `-` for stdin. Ragged text rows are padded.
        #[arg(required_unless_present = "art", conflicts_with = "art")]
        input: Option<PathBuf>,
        /// Animate a catalogue piece instead of a file.
        #[arg(long)]
        art: Option<String>,
        /// Play backwards, so the art leaves instead of arriving.
        #[arg(long)]
        reverse: bool,
        /// Total milliseconds, 100 to 60000.
        #[arg(long, default_value_t = 2000)]
        duration: u32,
        /// Frames per second, 1 to 60.
        #[arg(long, default_value_t = 20)]
        fps: u32,
        /// The same seed always gives the same frames.
        #[arg(long, default_value_t = 42)]
        seed: u32,
        /// Zero-based frame of a JSON project to animate.
        #[arg(long, default_value_t = 0)]
        source_frame: usize,
    },
    /// Load .txt or an ApollinaireASCII .json project. Text CRLF is normalised to LF.
    Import {
        input: PathBuf,
        #[arg(long)]
        pad: bool,
    },
    /// Overlay one static text frame onto another.
    Compose {
        background: PathBuf,
        overlay: PathBuf,
        #[arg(long, default_value_t = 0)]
        x: i32,
        #[arg(long, default_value_t = 0)]
        y: i32,
        #[arg(long)]
        transparent_spaces: bool,
    },
    /// Transform every frame in a project.
    Transform {
        input: PathBuf,
        #[arg(value_enum)]
        operation: Transform,
        #[arg(long, default_value_t = 0)]
        x: usize,
        #[arg(long, default_value_t = 0)]
        y: usize,
        #[arg(long)]
        width: Option<usize>,
        #[arg(long)]
        height: Option<usize>,
    },
    /// Play a project in a terminal. Escape codes are playback transport only.
    Play {
        input: PathBuf,
        #[arg(long,default_value_t=1,value_parser=clap::value_parser!(u32).range(1..=1000))]
        loops: u32,
    },
    /// Validate an existing text or JSON project.
    Check { input: PathBuf },
}
fn main() {
    if let Err(error) = run(Args::parse()) {
        eprintln!("apollinaire: {error}");
        std::process::exit(1);
    }
}
fn run(args: Args) -> Result<()> {
    let project = match args.command {
        Command::List { tag: Some(tag) } => {
            let mut found = false;
            for piece in art::search(&tag) {
                found = true;
                println!(
                    "{:<18} {:<11} {}",
                    piece.name,
                    piece.category,
                    piece.tags.join(", ")
                );
            }
            return if found {
                Ok(())
            } else {
                Err(format!("No artwork matches {tag}"))
            };
        }
        Command::List { tag: None } => {
            println!("Art ({} pieces):", art::CATALOGUE.len());
            for category in art::CATEGORIES {
                let names: Vec<_> = art::search("")
                    .filter(|p| p.category == *category)
                    .map(|p| p.name)
                    .collect();
                println!("  {category}: {}", names.join(", "));
            }
            println!(
                "Fonts: {}\nScenes: {}\nEffects: {}\nRamps: {}",
                lettering::FONTS.join(", "),
                animation::SCENES.join(", "),
                effects::EFFECTS.join(", "),
                apollinaire_ascii::image_renderer::RAMPS
                    .iter()
                    .map(|(n, _)| *n)
                    .collect::<Vec<_>>()
                    .join(", ")
            );
            return Ok(());
        }
        Command::Art { name, border } => {
            let frame = art::template(&name)?;
            Animation::still(if border { frame.border()? } else { frame })
        }
        Command::Text {
            text,
            font,
            font_file,
            align,
            width,
            wrap,
            horizontal,
            vertical,
        } => {
            let custom = font_file.as_ref().map(|p| read_text(p)).transpose()?;
            let layout = |layout| match layout {
                Layout::Default => lettering::Layout::Default,
                Layout::Full => lettering::Layout::Full,
                Layout::Fitted => lettering::Layout::Fitted,
                Layout::Smush => lettering::Layout::Smush,
            };
            let options = lettering::Options {
                alignment: match align {
                    Align::Left => lettering::Alignment::Left,
                    Align::Center => lettering::Alignment::Center,
                    Align::Right => lettering::Alignment::Right,
                },
                width,
                wrap,
                horizontal: layout(horizontal),
                vertical: layout(vertical),
            };
            Animation::still(lettering::render_with(
                &text,
                &font,
                custom.as_deref(),
                &options,
            )?)
        }
        Command::Image {
            inputs,
            output_dir,
            width,
            height,
            ramp,
            invert,
            dither,
            edges,
            edge_glyphs,
            gamma,
            contrast,
            cell_aspect,
            background,
        } => {
            let ramp = apollinaire_ascii::image_renderer::RAMPS
                .iter()
                .find(|(n, _)| *n == ramp)
                .map_or(ramp.clone(), |(_, r)| r.to_string());
            let options = RenderOptions {
                width,
                height,
                ramp,
                invert,
                dither,
                edges,
                edge_glyphs,
                gamma,
                contrast,
                cell_aspect,
                background,
            };
            options.validate()?;
            let convert = |input: &Path| {
                let bytes = match input.to_str().filter(|s| fetch::is_url(s)) {
                    Some(url) => fetch::image(url)?,
                    None => read_bounded(input)?,
                };
                apollinaire_ascii::decode_media(&bytes, &options)
            };
            if let Some(dir) = output_dir {
                return batch(&inputs, &dir, &args.export, convert);
            }
            if inputs.len() > 1 {
                return Err("Several inputs need --output-dir".into());
            }
            convert(&inputs[0])?
        }
        Command::Effect {
            effect,
            input,
            art,
            reverse,
            duration,
            fps,
            seed,
            source_frame,
        } => {
            let project = match (art, input) {
                (Some(name), _) => Animation::still(art::template(&name)?),
                (None, Some(path)) => load(&path, true)?,
                (None, None) => unreachable!("clap requires an input or --art"),
            };
            let source = &project
                .frames()
                .get(source_frame)
                .ok_or("--source-frame is outside the project")?
                .frame;
            effects::apply(
                &effect,
                source,
                &EffectOptions {
                    duration_ms: duration,
                    fps,
                    seed,
                    reverse,
                },
            )?
        }
        Command::Animate {
            scene,
            width,
            height,
            frames,
            fps,
            seed,
            text,
        } => animation::generate(
            &scene,
            &SceneOptions {
                width,
                height,
                frames,
                fps,
                seed,
                text,
            },
        )?,
        Command::Import { input, pad } => load(&input, pad)?,
        Command::Compose {
            background,
            overlay,
            x,
            y,
            transparent_spaces,
        } => {
            let base = load(&background, true)?;
            let stamp = load(&overlay, true)?;
            if base.frames().len() != 1 || stamp.frames().len() != 1 {
                return Err("Compose needs 2 static frames".into());
            }
            let mut canvas = Canvas::from_frame(&base.frames()[0].frame);
            canvas.overlay(&stamp.frames()[0].frame, x, y, transparent_spaces)?;
            Animation::still(canvas.finish())
        }
        Command::Transform {
            input,
            operation,
            x,
            y,
            width,
            height,
        } => {
            let project = load(&input, false)?;
            let mut frames = Vec::with_capacity(project.frames().len());
            for entry in project.frames() {
                let frame = match operation {
                    Transform::Horizontal => entry.frame.flip_horizontal(),
                    Transform::Vertical => entry.frame.flip_vertical(),
                    Transform::Border => entry.frame.border()?,
                    Transform::Crop => entry.frame.crop(
                        x,
                        y,
                        width.ok_or("Crop needs --width")?,
                        height.ok_or("Crop needs --height")?,
                    )?,
                };
                frames.push(TimedFrame {
                    frame,
                    duration_ms: entry.duration_ms,
                });
            }
            Animation::new(frames, project.looping())?
        }
        Command::Play { input, loops } => {
            if args.export.output.is_some()
                || args.export.format.is_some()
                || args.export.frame.is_some()
            {
                return Err("Play does not accept export options".into());
            }
            return play(&load(&input, false)?, loops);
        }
        Command::Check { input } => {
            let project = load(&input, false)?;
            println!(
                "Valid ASCII: {} x {}, {} frame(s), {} ms",
                project.width(),
                project.height(),
                project.frames().len(),
                project.duration_ms()
            );
            return Ok(());
        }
    };
    let (_, content) = export(&project, &args.export, args.export.output.as_deref())?;
    if let Some(path) = &args.export.output {
        create_new(path, &content)?;
    } else {
        io::stdout()
            .lock()
            .write_all(content.as_bytes())
            .map_err(|e| format!("Output failed: {e}"))?;
    }
    Ok(())
}
/// Choose the format from --format, the output extension or the frame count, and render it.
fn export(project: &Animation, args: &Export, output: Option<&Path>) -> Result<(Format, String)> {
    let format = args.format.unwrap_or_else(|| {
        match output.and_then(|p| p.extension()).and_then(|s| s.to_str()) {
            Some("json") => Format::Json,
            Some("html") => Format::Html,
            _ if project.frames().len() > 1 && args.frame.is_none() => Format::Json,
            _ => Format::Text,
        }
    });
    if args.frame.is_some() && !matches!(format, Format::Text) {
        return Err("--frame applies only to text output".into());
    }
    let content = match format {
        Format::Text => {
            if project.frames().len() > 1 && args.frame.is_none() {
                return Err("Animation needs JSON or HTML, or select --frame N for text".into());
            }
            let entry = project
                .frames()
                .get(args.frame.unwrap_or(0))
                .ok_or("Frame index is outside the project")?;
            format!("{}\n", entry.frame.text())
        }
        Format::Json => format!("{}\n", project.to_json()),
        Format::Html => export::html(project),
    };
    Ok((format, content))
}
fn create_new(path: &Path, content: &str) -> Result<()> {
    let mut file = File::options()
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(|e| format!("Cannot create {}: {e}", path.display()))?;
    file.write_all(content.as_bytes())
        .map_err(|e| format!("Write failed: {e}"))
}
/// Convert each input to a new file in `dir`. A failed input is reported and skipped;
/// existing files are never replaced.
fn batch(
    inputs: &[PathBuf],
    dir: &Path,
    args: &Export,
    convert: impl Fn(&Path) -> Result<Animation>,
) -> Result<()> {
    if args.output.is_some() {
        return Err("Use either --output or --output-dir".into());
    }
    let mut names: Vec<String> = Vec::with_capacity(inputs.len());
    for input in inputs {
        if input == Path::new("-") {
            return Err("Stdin cannot be used with --output-dir".into());
        }
        let name = output_name(input);
        // Case-insensitive file systems would put both inputs in one file.
        if names.iter().any(|n| n.eq_ignore_ascii_case(&name)) {
            return Err(format!(
                "Several inputs would be named {name} in {}; convert them separately",
                dir.display()
            ));
        }
        names.push(name);
    }
    std::fs::create_dir_all(dir).map_err(|e| format!("Cannot create {}: {e}", dir.display()))?;
    let mut failed = 0;
    for (input, stem) in inputs.iter().zip(&names) {
        let result = convert(input).and_then(|project| {
            let (format, content) = export(&project, args, None)?;
            let extension = match format {
                Format::Text => "txt",
                Format::Json => "json",
                Format::Html => "html",
            };
            let path = dir.join(format!("{stem}.{extension}"));
            create_new(&path, &content)?;
            Ok(path)
        });
        match result {
            Ok(path) => println!("{} -> {}", input.display(), path.display()),
            Err(error) => {
                failed += 1;
                eprintln!("apollinaire: {}: {error}", input.display());
            }
        }
    }
    if failed > 0 {
        return Err(format!("{failed} of {} inputs failed", inputs.len()));
    }
    Ok(())
}
/// The input's file name, or the last segment of a URL path, as `[A-Za-z0-9._-]`. Keeping its
/// extension lets `clip.png` and `clip.gif` share a batch as `clip.png.txt` and `clip.gif.json`.
fn output_name(input: &Path) -> String {
    let text = input.to_string_lossy();
    let name = if fetch::is_url(&text) {
        let path = text.split(['?', '#']).next().unwrap_or_default();
        path.splitn(4, '/')
            .nth(3)
            .unwrap_or_default()
            .rsplit('/')
            .next()
            .unwrap_or_default()
            .to_string()
    } else {
        input
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default()
    };
    let clean: String = name
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || "._-".contains(c) {
                c
            } else {
                '_'
            }
        })
        .collect();
    if clean.trim_matches(['.', '_']).is_empty() {
        "image".into()
    } else {
        clean
    }
}
fn read_bounded(path: &Path) -> Result<Vec<u8>> {
    let mut bytes = Vec::new();
    let reader: Box<dyn Read> = if path == Path::new("-") {
        Box::new(io::stdin())
    } else {
        Box::new(File::open(path).map_err(|e| format!("Cannot read {}: {e}", path.display()))?)
    };
    reader
        .take(MAX_INPUT_BYTES as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| format!("Read failed: {e}"))?;
    if bytes.len() > MAX_INPUT_BYTES {
        return Err("Input exceeds 16 MiB".into());
    }
    Ok(bytes)
}
fn read_text(path: &Path) -> Result<String> {
    String::from_utf8(read_bounded(path)?)
        .map_err(|_| "Text input must be UTF-8 containing printable ASCII".into())
}
fn load(path: &Path, pad: bool) -> Result<Animation> {
    let text = read_text(path)?;
    if path.extension().is_some_and(|e| e == "json")
        || (!path.extension().is_some_and(|e| e == "txt")
            && text.trim_start().starts_with('{')
            && serde_json::from_str::<serde::de::IgnoredAny>(&text).is_ok())
    {
        Animation::from_json(&text)
    } else {
        let text = text.replace("\r\n", "\n");
        Ok(Animation::still(if pad {
            Frame::from_text_padded(&text)?
        } else {
            Frame::from_text(&text)?
        }))
    }
}
fn play(project: &Animation, loops: u32) -> Result<()> {
    let stdout = io::stdout();
    if !stdout.is_terminal() {
        return Err(
            "Playback needs a terminal; export JSON or HTML when redirecting output".into(),
        );
    }
    let mut output = stdout.lock();
    output
        .write_all(b"\x1b[2J\x1b[H")
        .map_err(|e| e.to_string())?;
    for _ in 0..loops {
        for entry in project.frames() {
            write!(output, "\x1b[H{}", entry.frame.text()).map_err(|e| e.to_string())?;
            output.flush().map_err(|e| e.to_string())?;
            std::thread::sleep(Duration::from_millis(u64::from(entry.duration_ms)));
        }
    }
    writeln!(output).map_err(|e| e.to_string())
}
