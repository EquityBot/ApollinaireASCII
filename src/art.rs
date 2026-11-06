// Copyright 2026 Ryan Duguid
// SPDX-License-Identifier: MIT

//! A catalogue of original ASCII artwork, grouped by category and searchable by tag.
//!
//! Every piece was drawn for this repository; no artwork from other projects is copied.

use crate::{Frame, Result};

/// One catalogue entry. `text` may have ragged rows; [`Piece::frame`] pads them.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Piece {
    pub name: &'static str,
    pub category: &'static str,
    pub tags: &'static [&'static str],
    pub text: &'static str,
}

impl Piece {
    pub fn frame(&self) -> Result<Frame> {
        Frame::from_text_padded(self.text)
    }
    /// True when the query is empty or appears in the name, category or a tag, ignoring case.
    pub fn matches(&self, query: &str) -> bool {
        let query = query.trim().to_ascii_lowercase();
        query.is_empty()
            || self.name.contains(&query)
            || self.category.contains(&query)
            || self.tags.iter().any(|tag| tag.contains(&query))
    }
}

pub const CATEGORIES: &[&str] = &[
    "animals",
    "nature",
    "objects",
    "characters",
    "borders",
    "emoticons",
    "banners",
];

pub fn piece(name: &str) -> Option<&'static Piece> {
    CATALOGUE.iter().find(|p| p.name == name)
}

/// Pieces matching `query` in catalogue order; see [`Piece::matches`].
pub fn search(query: &str) -> impl Iterator<Item = &'static Piece> {
    CATALOGUE.iter().filter(move |p| p.matches(query))
}

pub fn template(name: &str) -> Result<Frame> {
    piece(name)
        .ok_or_else(|| format!("Unknown artwork {name}; run `apollinaire list` for the catalogue"))?
        .frame()
}

macro_rules! art {
    ($name:literal, $category:literal, [$($tag:literal),*], $text:expr) => {
        Piece { name: $name, category: $category, tags: &[$($tag),*], text: $text }
    };
}

pub const CATALOGUE: &[Piece] = &[
    // Animals
    art!(
        "cat",
        "animals",
        ["pet", "kitten", "mammal"],
        r"     /\   /\
    (  . .  )
     \  ^  /
    /|     |\
   (_|_____|_)~~"
    ),
    art!(
        "dog",
        "animals",
        ["pet", "puppy", "mammal"],
        r"   __      __
  /  \.--./  \
  \_/ o  o \_/
    |  ..  |
    \  \/  /
     '----'"
    ),
    art!(
        "rabbit",
        "animals",
        ["bunny", "pet", "mammal", "easter"],
        r"   _      _
  ( \    / )
   \ \  / /
   ( o  o )
  =(  ..  )=
   (_)--(_)"
    ),
    art!(
        "bear",
        "animals",
        ["teddy", "mammal", "forest"],
        r"  .-.     .-.
 ( (_____) )
  / o   o \
 |    ^    |
 |  \___/  |
  \_______/"
    ),
    art!(
        "mouse",
        "animals",
        ["pet", "rodent", "mammal"],
        r"  .--.     .--.
 (    )___(    )
  '--/ o o \--'
    =\  v  /=
      '---'"
    ),
    art!(
        "pig",
        "animals",
        ["farm", "mammal"],
        r"    ,-.___.-,
   /  ^   ^  \
  |  .-----.  |
  |  | o o |  |
   \ '-----' /
    '-.___.-'
     ||   ||"
    ),
    art!(
        "duck",
        "animals",
        ["bird", "pond", "water"],
        r"      ,--.
     ( o  >
      |  |
  .---'  '---.
   \  ~~~~  /
    '------'
 ~~~~~~~~~~~~~~"
    ),
    art!(
        "penguin",
        "animals",
        ["bird", "ice", "winter"],
        r"     .---.
    / o o \
    |  v  |
   /|     |\
  / |     | \
    \     /
    _|___|_"
    ),
    art!(
        "owl",
        "animals",
        ["bird", "night", "wise"],
        r"     ,___,
    / o o \
   (   V   )
   /|     |\
  (_|_____|_)
     / | \"
    ),
    art!(
        "bird",
        "animals",
        ["songbird", "tweet"],
        r"     .-.
    (o  >
    / \
   /   \__
  /______/
     ||
    _||_"
    ),
    art!(
        "fish",
        "animals",
        ["sea", "water", "ocean"],
        r"       .------.
  ><  /  o     \
   \\/       __/
   /\  _____/
  /  \/ "
    ),
    art!(
        "whale",
        "animals",
        ["sea", "ocean", "water", "mammal"],
        r"       '  '
        \/
   .----'---.         __
  /  o       '-.____.' /
 (                    <
  '-.______________.'\_\
 ~~~~~~~~~~~~~~~~~~~~~~~~~"
    ),
    art!(
        "octopus",
        "animals",
        ["sea", "ocean", "tentacles"],
        r"     .---.
    / o o \
    \  -  /
   //|||||\\
  // ||||| \\
 ~  ~ ~ ~ ~  ~"
    ),
    art!(
        "crab",
        "animals",
        ["sea", "beach", "claws"],
        r"  (\/)     (\/)
   \ \_____/ /
    \ o   o /
    / \___/ \
   /|  | |  |\"
    ),
    art!(
        "turtle",
        "animals",
        ["reptile", "shell", "slow"],
        r"        ________
   __  / \__/\__\
  (o )/ _/  \_  \
   \__\/  \__/ \_/_
      '-'     '-'"
    ),
    art!(
        "snail",
        "animals",
        ["shell", "slow", "garden"],
        r"   o  o     ____
    \ \    / __ \
     \ \__/ /  \ |
     (o_____\__/ /
      '---------'"
    ),
    art!(
        "frog",
        "animals",
        ["pond", "amphibian", "green"],
        r"    .-.   .-.
   ( o )-( o )
  /           \
 (   \_____/   )
  \           /
 __)  )   (  (__
(____/     \____)"
    ),
    art!(
        "snake",
        "animals",
        ["reptile", "serpent"],
        r"     ____
    / o  \____-<
    \  ________/
     \ \
  ____\ \____
 (___________)"
    ),
    art!(
        "fox",
        "animals",
        ["forest", "mammal", "clever"],
        r"  /\       /\
 /  \_____/  \
 \  /     \  /
  \/ o   o \/
   \   v   /
    \_____/"
    ),
    art!(
        "elephant",
        "animals",
        ["mammal", "safari", "trunk"],
        r"   .--.  ____  .--.
  /    \/    \/    \
 |     | o  o |     |
  \    |  ||  |    /
   '--'|  ||  |'--'
       '. || .'
         \||/
          ()"
    ),
    art!(
        "bat",
        "animals",
        ["night", "halloween", "wings"],
        r" __    /\__/\    __
 \ '-./ o  o \.-' /
  \   \  vv  /   /
   '._/'----'\_.'"
    ),
    art!(
        "spider",
        "animals",
        ["bug", "web", "halloween"],
        r"       |
       |
   \ .-^-. /
  --( o o )--
   / \___/ \
  /  /   \  \"
    ),
    art!(
        "butterfly",
        "animals",
        ["bug", "insect", "wings", "garden"],
        r"   __      __
  /  \.  ./  \
 |  o  \/  o  |
  \__  ||  __/
  /  \ || /  \
 |  o  ||  o  |
  \__/    \__/"
    ),
    art!(
        "bee",
        "animals",
        ["bug", "insect", "honey", "garden"],
        r"       __   __
      (  \ /  )
   ___ \  V  /
  (o  |=|=|=|>
   '--'-'-'-'"
    ),
    // Nature and sky
    art!(
        "mountain",
        "nature",
        ["landscape", "peak", "outdoors"],
        r"              /\
       /\    /  \      /\
      /  \  / /\ \    /  \
  ___/____\/_/__\_\__/____\___
 ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~"
    ),
    art!(
        "tree",
        "nature",
        ["forest", "pine", "christmas"],
        r"        /\
       /**\
      /****\
     /******\
    /********\
       |  |
    ___|__|___"
    ),
    art!(
        "flower",
        "nature",
        ["garden", "plant", "spring"],
        r"      .-.
   .-(   )-.
  (   \ /   )
   '-.( ).-'
      /|\
       |
   \   |   /
    \__|__/
       |"
    ),
    art!(
        "cactus",
        "nature",
        ["desert", "plant"],
        r"       .-.
       | |  .-.
  .-.  | |  | |
  | |  | |__| |
  | |__|  ____/
  '----.  |
       |  |
   ____|__|____"
    ),
    art!(
        "mushroom",
        "nature",
        ["forest", "fungus"],
        r"     .-'~~~'-.
   .'  o   o  '.
  /  o    O    o\
  '-.__________.-'
       |    |
       |    |
      (______)"
    ),
    art!(
        "island",
        "nature",
        ["palm", "beach", "sea", "holiday"],
        r"         \|/
       -- @ --
         /|\
          |
          |
    ______|______
 ~~~~~~~~~~~~~~~~~~~"
    ),
    art!(
        "sun",
        "nature",
        ["sky", "weather", "summer", "day"],
        r"       \  |  /
     '. .---. .'
   --- (     ) ---
     .' '---' '.
       /  |  \"
    ),
    art!(
        "moon",
        "nature",
        ["sky", "night", "crescent"],
        r"     .--.
   .'   /
  /   .'
 |   (
  \   '.
   '.   \
     '--'"
    ),
    art!(
        "cloud",
        "nature",
        ["sky", "weather"],
        r"        .--.
     .-(    )-.
   .(          ).
  (______________)"
    ),
    art!(
        "storm",
        "nature",
        ["sky", "weather", "rain", "lightning"],
        r"        .--.
     .-(    )-.
   .(          ).
  (______________)
    ' ' /  /' ' '
   ' ' /_ /' ' '
        //
        /"
    ),
    art!(
        "snowflake",
        "nature",
        ["winter", "snow", "weather", "christmas"],
        r"     .  |  .
   \  \ | /  /
  -- --( )-- --
   /  / | \  \
     '  |  '"
    ),
    art!(
        "planet",
        "nature",
        ["space", "saturn", "rings"],
        r"        .---.
   .---/     \---.
  (   |       |   )
   '---\     /---'
        '---'"
    ),
    art!(
        "comet",
        "nature",
        ["space", "sky", "star"],
        r"   .    .   .  .--.
     '   .  .-'  @@)
  -  - ===<     @@@)
     .   '  '-.  @@)
   '    .   .  '--'"
    ),
    // Objects
    art!(
        "rocket",
        "objects",
        ["space", "launch", "ship"],
        r"       /\
      /  \
     | () |
     |    |
    /|    |\
   /_|____|_\
      /\/\
       \/ "
    ),
    art!(
        "castle",
        "objects",
        ["building", "fairytale", "tower"],
        r"   _   _       _   _
  | |_| |     | |_| |
  |     |_____|     |
  |  []         []  |
  |       ___       |
  |      /   \      |
 _|______|___|______|_"
    ),
    art!(
        "house",
        "objects",
        ["building", "home"],
        r"          ( )
        __|_|______
       /          /\
      /          /  \
     /__________/ [] \
     | []  []  | __  |
     |_________||  |_|"
    ),
    art!(
        "heart",
        "objects",
        ["love", "valentine", "symbol"],
        r"    __    __
  /    \/    \
 |            |
  \          /
   \        /
    \      /
     \    /
      \  /
       \/"
    ),
    art!(
        "coffee",
        "objects",
        ["drink", "cup", "mug", "morning"],
        r"      (  )
       )(
    .------.
    |      |__
    |      |  )
    |______|_/
   ==========="
    ),
    art!(
        "teapot",
        "objects",
        ["drink", "tea", "kitchen"],
        r"          _
     .---(_)---.
  _ /           \ __
 ( |             |  )
  \|             |_/
    \___________/"
    ),
    art!(
        "lightbulb",
        "objects",
        ["idea", "lamp", "light"],
        r"     .---.
    /     \
   |  ~~~  |
    \  |  /
     |===|
     |===|
      '-'"
    ),
    art!(
        "candle",
        "objects",
        ["light", "flame", "birthday"],
        r"      (
       )
      (_)
     .-|-.
     |   |
     |   |
     |   |
   __|___|__
  '---------'"
    ),
    art!(
        "key",
        "objects",
        ["lock", "door", "security"],
        r"   .--.
  /    \__________
 |  ()  _____   _|
  \    /     |_|
   '--'"
    ),
    art!(
        "envelope",
        "objects",
        ["mail", "letter", "message"],
        r"  +----------------+
  |\              /|
  | '.          .' |
  |   '-.____.-'   |
  |                |
  +----------------+"
    ),
    art!(
        "book",
        "objects",
        ["reading", "library", "story"],
        r"   _______  _______
  /       \/       \
 | ~~~~~~ || ~~~~~~ |
 | ~~~~~~ || ~~~~~~ |
 | ~~~~~~ || ~~~~~~ |
 |________||________|
  '-------''-------'"
    ),
    art!(
        "clock",
        "objects",
        ["time", "watch"],
        r"     .-----.
   .'  12   '.
  |     |     |
  |9    o--  3|
  |           |
   '.   6   .'
     '-----'"
    ),
    art!(
        "umbrella",
        "objects",
        ["rain", "weather"],
        r"       .-'``'-.
     .'        '.
    /            \
   '-.-'-.-'-.-'-'
         |
         |
       \_/"
    ),
    art!(
        "anchor",
        "objects",
        ["sea", "boat", "nautical"],
        r"      .-.
     ( o )
      | |
   ---+-+---
      | |
  \   | |   /
   \__| |__/
    '-----'"
    ),
    art!(
        "sailboat",
        "objects",
        ["sea", "boat", "nautical", "holiday"],
        r"         |\
         | \
         |  \
         |___\
     ____|________
     \  o o o    /
  ~~~~~~~~~~~~~~~~~~~"
    ),
    art!(
        "car",
        "objects",
        ["vehicle", "road", "drive"],
        r"        _______
    ___/  | |  \___
   |    __|_|__    |
   '-(@)-------(@)-'"
    ),
    art!(
        "gift",
        "objects",
        ["present", "birthday", "christmas", "party"],
        r"      __  __
     (  \/  )
  .---\    /---.
  |    ||||    |
  |----||||----|
  |    ||||    |
  '----''''----'"
    ),
    art!(
        "balloon",
        "objects",
        ["party", "birthday", "celebration"],
        r"     .---.
    /     \
   |       |
    \     /
     '. .'
       Y
       |
        )
       ("
    ),
    art!(
        "trophy",
        "objects",
        ["award", "winner", "cup", "celebration"],
        r"   .-----------.
  (|           |)
   \    WIN    /
    '.       .'
      '-. .-'
        | |
      .-' '-.
     |_______|"
    ),
    art!(
        "camera",
        "objects",
        ["photo", "picture"],
        r"     .-----.
  .--' [_] '----.
  |    .---.  o |
  |   / .-. \   |
  |   \ '-' /   |
  |    '---'    |
  '-------------'"
    ),
    art!(
        "computer",
        "objects",
        ["laptop", "terminal", "code", "tech"],
        r"   .------------.
   | >_         |
   |            |
   |            |
   '------------'
       _|__|_
   .-------------.
  /  ::::::::::  \
 '----------------'"
    ),
    // Characters
    art!(
        "ghost",
        "characters",
        ["spooky", "halloween", "boo"],
        r"       .------.
     .'        '.
    /   o    o   \
   |      __      |
   |              |
   |              |
   /_/\__/\__/\__/\"
    ),
    art!(
        "robot",
        "characters",
        ["machine", "tech", "android"],
        r"       _|_
    .-------.
    | [] [] |
    |  ===  |
  __|_______|__
 /  |       |  \
    |_______|
     |     |
    _|     |_"
    ),
    art!(
        "alien",
        "characters",
        ["space", "ufo", "visitor"],
        r"     .     .
      \   /
    .-'---'-.
   /  (o)(o) \
   \    ~    /
    '-.___.-'
     /|   |\"
    ),
    art!(
        "snowman",
        "characters",
        ["winter", "snow", "christmas"],
        r"       ___
     _|___|_
      (o o)
      ( > )
    _(  :  )_
   (    :    )
    '-------'"
    ),
    // Borders and dividers
    art!(
        "frame-double",
        "borders",
        ["frame", "box", "border"],
        r"#======================#
||                    ||
||                    ||
||                    ||
#======================#"
    ),
    art!(
        "frame-rounded",
        "borders",
        ["frame", "box", "border"],
        r" .--------------------.
|                      |
|                      |
|                      |
 '--------------------'"
    ),
    art!(
        "frame-stars",
        "borders",
        ["frame", "box", "border", "sparkle"],
        r"*~*~*~*~*~*~*~*~*~*~*~*
*                     *
~                     ~
*                     *
*~*~*~*~*~*~*~*~*~*~*~*"
    ),
    art!(
        "frame-ornate",
        "borders",
        ["frame", "box", "border", "fancy"],
        r".-=~=-.-=~=-.-=~=-.-=~=-.
(                        )
 )                      (
(                        )
'-=~=-'-=~=-'-=~=-'-=~=-'"
    ),
    art!(
        "frame-scroll",
        "borders",
        ["frame", "parchment", "border", "fancy"],
        r"  _____________________
 / \                   \
|   |                   |
 \_ |                   |
    |                   |
    |   ________________|_
     \_/__________________/"
    ),
    art!(
        "divider-wave",
        "borders",
        ["divider", "line", "separator"],
        r" .-'-.   .-'-.   .-'-.   .-'-.   .-'-.
'     '-'     '-'     '-'     '-'     '"
    ),
    art!(
        "divider-diamonds",
        "borders",
        ["divider", "line", "separator"],
        r"--<>--<>--<>--<>--<>--<>--<>--<>--<>--"
    ),
    art!(
        "divider-arrows",
        "borders",
        ["divider", "line", "separator", "direction"],
        r">>>----->>>----->>>----->>>----->>>"
    ),
    art!(
        "divider-chain",
        "borders",
        ["divider", "line", "separator"],
        r"=(o)=(o)=(o)=(o)=(o)=(o)=(o)=(o)=(o)="
    ),
    art!(
        "divider-flourish",
        "borders",
        ["divider", "line", "separator", "fancy"],
        r"-=-=-=-=-=-=-=-<( * )>-=-=-=-=-=-=-=-"
    ),
    // Emoticons
    art!("happy", "emoticons", ["smile", "face", "joy"], r"(^_^)"),
    art!("cheer", "emoticons", ["hooray", "face", "joy"], r"\(^o^)/"),
    art!(
        "wave-hello",
        "emoticons",
        ["hello", "greeting", "face"],
        r"(^_^)/"
    ),
    art!("wink", "emoticons", ["face", "playful"], r"(^_-)"),
    art!(
        "sleepy",
        "emoticons",
        ["tired", "face", "sleep"],
        r"(-_-) zzZ"
    ),
    art!(
        "surprised",
        "emoticons",
        ["shock", "face", "wow"],
        r"(O_O)!"
    ),
    art!("sad", "emoticons", ["cry", "face", "tears"], r"(T_T)"),
    art!("shrug", "emoticons", ["dunno", "face"], r"\_(o_o)_/"),
    art!(
        "love",
        "emoticons",
        ["heart", "face", "blush"],
        r"(*^_^*) <3"
    ),
    art!("dizzy", "emoticons", ["confused", "face"], r"(@_@)"),
    art!(
        "dance",
        "emoticons",
        ["party", "face", "music"],
        r"<(^_^<)  (>^_^)>"
    ),
    // Banners
    art!(
        "welcome",
        "banners",
        ["sign", "greeting", "hello"],
        r"  .---------------------.
 /|                     |\
< |    W E L C O M E    | >
 \|_____________________|/"
    ),
    art!(
        "thank-you",
        "banners",
        ["sign", "thanks", "gratitude"],
        r" .-~-~-~-~-~-~-~-~-~-~-~-.
(     T H A N K   Y O U   )
 '-~-~-~-~-~-~-~-~-~-~-~-'"
    ),
    art!(
        "congratulations",
        "banners",
        ["sign", "celebration", "party", "winner"],
        r" *  .  *  .  *  .  *  .  *  .  *
  +---------------------------+
  |  CONGRATULATIONS!  \o/    |
  +---------------------------+
 *  .  *  .  *  .  *  .  *  .  *"
    ),
    art!(
        "birthday",
        "banners",
        ["cake", "party", "celebration", "sign"],
        r"       i  i  i  i
      _|__|__|__|_
     |  ~  ~  ~  |
    _|___________|_
   |               |
   |HAPPY BIRTHDAY!|
   |_______________|"
    ),
    art!(
        "the-end",
        "banners",
        ["sign", "finale", "credits"],
        r"  ____________________
 |                    |
 |    ~ THE  END ~    |
 |____________________|"
    ),
    art!(
        "present-day",
        "banners",
        ["sign", "time", "clock"],
        r"  [===============================]
  |   PRESENT DAY, PRESENT TIME   |
  [===============================]"
    ),
];
