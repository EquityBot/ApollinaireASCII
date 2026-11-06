// Copyright 2026 Ryan Duguid
// SPDX-License-Identifier: MIT

use std::collections::BTreeSet;

const RULE: &str = "============================================================";

/// `name version` of every locked package that has a source, so the root crate is excluded.
fn locked_packages(lock: &str) -> BTreeSet<String> {
    let mut packages = BTreeSet::new();
    for block in lock.split("[[package]]").skip(1) {
        let field = |key: &str| {
            block.lines().find_map(|line| {
                line.strip_prefix(key)?
                    .strip_prefix(" = \"")?
                    .strip_suffix('"')
            })
        };
        let name = field("name").expect("locked package without a name");
        let version = field("version").expect("locked package without a version");
        if field("source").is_some() {
            packages.insert(format!("{name} {version}"));
        }
    }
    packages
}

/// A `name version` heading, as opposed to free text after a rule inside a licence.
fn is_package_heading(heading: &str) -> bool {
    heading.split_once(' ').is_some_and(|(name, version)| {
        !name.is_empty()
            && name
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
            && version.starts_with(|c: char| c.is_ascii_digit())
            && !version.contains(' ')
    })
}

struct Sections {
    headings: BTreeSet<String>,
    duplicates: Vec<String>,
    without_licence: Vec<String>,
}

/// Package sections: a rule, then a `name version` heading, then `Licence:`. Sections for code
/// or fonts copied from outside Cargo give `Source:` under the heading instead and are skipped.
fn notice_sections(notices: &str) -> Sections {
    let mut sections = Sections {
        headings: BTreeSet::new(),
        duplicates: Vec::new(),
        without_licence: Vec::new(),
    };
    let lines: Vec<_> = notices.lines().collect();
    for window in lines.windows(3) {
        let [RULE, heading, next] = window else {
            continue;
        };
        if next.starts_with("Source: ") || !is_package_heading(heading) {
            continue;
        }
        if !next.starts_with("Licence: ") {
            sections.without_licence.push(heading.to_string());
        }
        if !sections.headings.insert(heading.to_string()) {
            sections.duplicates.push(heading.to_string());
        }
    }
    sections
}

// Kim Wexler would want every licence on file before anything ships.
#[test]
fn notice_bundle_has_one_section_per_locked_package() {
    let root = env!("CARGO_MANIFEST_DIR");
    let lock = std::fs::read_to_string(format!("{root}/Cargo.lock")).unwrap();
    let notices = std::fs::read_to_string(format!("{root}/web/THIRD_PARTY_NOTICES.txt")).unwrap();
    let locked = locked_packages(&lock);
    let Sections {
        headings,
        duplicates,
        without_licence,
    } = notice_sections(&notices);
    let missing: Vec<_> = locked.difference(&headings).collect();
    let stale: Vec<_> = headings.difference(&locked).collect();
    assert!(
        missing.is_empty()
            && stale.is_empty()
            && duplicates.is_empty()
            && without_licence.is_empty(),
        "web/THIRD_PARTY_NOTICES.txt is out of date with Cargo.lock; see THIRD_PARTY_NOTICES.md\n\
         missing sections: {missing:?}\nsections for unlocked packages: {stale:?}\n\
         duplicate sections: {duplicates:?}\nsections without a Licence line: {without_licence:?}"
    );
}
