// Copyright 2026 Ryan Duguid
// SPDX-License-Identifier: MIT

use apollinaire_ascii::{Animation, Canvas, Result, SceneOptions, animation};

fn main() -> Result<()> {
    let mut canvas = Canvas::new(24, 6)?;
    canvas.rectangle(0, 0, 24, 6, b'#')?;
    canvas.text(3, 2, "HELLO, FRIEND")?;
    println!("{}", canvas.finish().text());
    let project = animation::generate("ghost", &SceneOptions::default())?;
    assert_eq!(Animation::from_json(&project.to_json())?, project);
    Ok(())
}
