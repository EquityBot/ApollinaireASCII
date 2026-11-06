// Copyright 2026 Ryan Duguid
// SPDX-License-Identifier: MIT

//! Validate while deserialising, before an untrusted JSON tree can grow.
use crate::{Animation, Frame, MAX_FRAMES, MAX_PROJECT_CELLS, Result, TimedFrame, dimensions};
use serde::de::{self, Deserialize, Deserializer, IgnoredAny, MapAccess, SeqAccess, Visitor};
use std::fmt;

pub(crate) fn parse(input: &str) -> Result<Animation> {
    serde_json::from_str::<Project>(input)
        .map(|project| project.0)
        .map_err(|error| format!("Invalid project: {error}"))
}

struct Project(Animation);
impl<'de> Deserialize<'de> for Project {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> std::result::Result<Self, D::Error> {
        struct ProjectVisitor;
        impl<'de> Visitor<'de> for ProjectVisitor {
            type Value = Project;
            fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str("an asciiforge version 1 project")
            }
            fn visit_map<M: MapAccess<'de>>(
                self,
                mut map: M,
            ) -> std::result::Result<Project, M::Error> {
                let (mut format, mut version, mut width, mut height, mut looping, mut frames) =
                    (None, None, None, None, None, None);
                while let Some(key) = map.next_key::<String>()? {
                    match key.as_str() {
                        "format" => field(&mut format, &mut map, "format")?,
                        "version" => field(&mut version, &mut map, "version")?,
                        "width" => field(&mut width, &mut map, "width")?,
                        "height" => field(&mut height, &mut map, "height")?,
                        "loop" => field(&mut looping, &mut map, "loop")?,
                        "frames" => field(&mut frames, &mut map, "frames")?,
                        _ => {
                            return Err(de::Error::unknown_field(
                                &key,
                                &["format", "version", "width", "height", "loop", "frames"],
                            ));
                        }
                    }
                }
                if required::<String, M::Error>(format, "format")? != "asciiforge"
                    || required::<u32, M::Error>(version, "version")? != 1
                {
                    return Err(de::Error::custom("Expected asciiforge project version 1"));
                }
                let width = required::<usize, M::Error>(width, "width")?;
                let height = required::<usize, M::Error>(height, "height")?;
                dimensions(width, height).map_err(de::Error::custom)?;
                let looping = required::<bool, M::Error>(looping, "loop")?;
                let frames = required::<Frames, M::Error>(frames, "frames")?.0;
                if frames
                    .iter()
                    .any(|entry| entry.frame.width() != width || entry.frame.height() != height)
                {
                    return Err(de::Error::custom(
                        "Frame dimensions do not match the project",
                    ));
                }
                Animation::new(frames, looping)
                    .map(Project)
                    .map_err(de::Error::custom)
            }
        }
        deserializer.deserialize_map(ProjectVisitor)
    }
}

fn field<'de, T: Deserialize<'de>, M: MapAccess<'de>>(
    slot: &mut Option<T>,
    map: &mut M,
    name: &'static str,
) -> std::result::Result<(), M::Error> {
    if slot.is_some() {
        return Err(de::Error::duplicate_field(name));
    }
    *slot = Some(map.next_value()?);
    Ok(())
}
fn required<T, E: de::Error>(value: Option<T>, name: &'static str) -> std::result::Result<T, E> {
    value.ok_or_else(|| E::missing_field(name))
}

struct Frames(Vec<TimedFrame>);
impl<'de> Deserialize<'de> for Frames {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> std::result::Result<Self, D::Error> {
        struct FramesVisitor;
        impl<'de> Visitor<'de> for FramesVisitor {
            type Value = Frames;
            fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str("1 to 600 bounded frames")
            }
            fn visit_seq<S: SeqAccess<'de>>(
                self,
                mut sequence: S,
            ) -> std::result::Result<Frames, S::Error> {
                let mut frames = Vec::new();
                let mut cells = 0usize;
                while frames.len() < MAX_FRAMES {
                    let Some(entry) = sequence.next_element::<Entry>()? else {
                        return Ok(Frames(frames));
                    };
                    cells += entry.0.frame.cells().len();
                    if cells > MAX_PROJECT_CELLS {
                        return Err(de::Error::custom("Project exceeds 8,000,000 cells"));
                    }
                    frames.push(entry.0);
                }
                if sequence.next_element::<IgnoredAny>()?.is_some() {
                    return Err(de::Error::custom("Too many frames"));
                }
                Ok(Frames(frames))
            }
        }
        deserializer.deserialize_seq(FramesVisitor)
    }
}

struct Entry(TimedFrame);
impl<'de> Deserialize<'de> for Entry {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> std::result::Result<Self, D::Error> {
        struct EntryVisitor;
        impl<'de> Visitor<'de> for EntryVisitor {
            type Value = Entry;
            fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str("a frame with text and duration_ms")
            }
            fn visit_map<M: MapAccess<'de>>(
                self,
                mut map: M,
            ) -> std::result::Result<Entry, M::Error> {
                let (mut text, mut duration) = (None, None);
                while let Some(key) = map.next_key::<String>()? {
                    match key.as_str() {
                        "text" => field(&mut text, &mut map, "text")?,
                        "duration_ms" => field(&mut duration, &mut map, "duration_ms")?,
                        _ => return Err(de::Error::unknown_field(&key, &["text", "duration_ms"])),
                    }
                }
                let frame = required::<Text, M::Error>(text, "text")?.0;
                let duration_ms = required::<u32, M::Error>(duration, "duration_ms")?;
                if !(10..=60_000).contains(&duration_ms) {
                    return Err(de::Error::custom(
                        "Frame duration must be 10..=60000 milliseconds",
                    ));
                }
                Ok(Entry(TimedFrame { frame, duration_ms }))
            }
        }
        deserializer.deserialize_map(EntryVisitor)
    }
}

struct Text(Frame);
impl<'de> Deserialize<'de> for Text {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> std::result::Result<Self, D::Error> {
        struct TextVisitor;
        impl Visitor<'_> for TextVisitor {
            type Value = Text;
            fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str("rectangular printable ASCII text")
            }
            fn visit_str<E: de::Error>(self, text: &str) -> std::result::Result<Text, E> {
                Frame::from_text(text).map(Text).map_err(de::Error::custom)
            }
        }
        deserializer.deserialize_str(TextVisitor)
    }
}
