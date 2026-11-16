//! DESIGN.md, PLAN.md, walkthrough.html and judge-eval/ moved into this
//! folder together in slice 1. Their relative links must still resolve.

use std::fs;
use std::path::Path;

/// The relative targets of a markdown file's `[text](target)` links, without
/// any `#section` part.
fn relative_links(markdown: &str) -> Vec<String> {
    relative_links_with_anchors(markdown)
        .into_iter()
        .map(|(path, _)| path)
        .collect()
}

/// The same links, each with its `#section` part, if it has one.
fn relative_links_with_anchors(markdown: &str) -> Vec<(String, Option<String>)> {
    let mut links = Vec::new();
    let mut rest = markdown;
    while let Some(start) = rest.find("](") {
        rest = &rest[start + 2..];
        let Some(end) = rest.find(')') else {
            break;
        };
        let target = &rest[..end];
        rest = &rest[end..];
        if target.contains("://") || target.starts_with('#') || target.starts_with("mailto:") {
            continue;
        }
        let (path, anchor) = match target.split_once('#') {
            Some((path, anchor)) => (path, Some(anchor.to_string())),
            None => (target, None),
        };
        links.push((path.to_string(), anchor));
    }
    links
}

/// The lines a GitHub-style line anchor names: `L12` is (12, 12) and
/// `L12-L30` is (12, 30). `None` for any other anchor, such as a heading.
fn line_range(anchor: &str) -> Option<(usize, usize)> {
    let rest = anchor.strip_prefix('L')?;
    let (first, last) = match rest.split_once("-L") {
        Some((first, last)) => (first, last),
        None => (rest, rest),
    };
    Some((first.parse().ok()?, last.parse().ok()?))
}

#[test]
fn design_doc_still_links_to_the_files_moved_with_it() {
    let crate_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let design = fs::read_to_string(crate_root.join("DESIGN.md")).unwrap();
    let links = relative_links(&design);
    for expected in ["judge-eval/", "PLAN.md", "walkthrough.html"] {
        assert!(
            links.iter().any(|link| link == expected),
            "DESIGN.md no longer links to {expected}"
        );
    }
}

/// The docs whose links are checked: the ones at the crate root, judge-eval's
/// README, and every markdown file in `docs/`, so a new doc there is checked
/// without being listed here.
fn docs() -> Vec<String> {
    let crate_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut docs: Vec<String> = ["DESIGN.md", "PLAN.md", "CLAUDE.md", "judge-eval/README.md"]
        .map(String::from)
        .to_vec();
    let mut in_docs: Vec<String> = fs::read_dir(crate_root.join("docs"))
        .unwrap()
        .map(|entry| entry.unwrap().file_name().into_string().unwrap())
        .filter(|name| name.ends_with(".md"))
        .map(|name| format!("docs/{name}"))
        .collect();
    in_docs.sort();
    docs.extend(in_docs);
    docs
}

#[test]
fn the_docs_folder_is_checked() {
    let docs = docs();
    assert!(
        docs.contains(&"docs/slice-1-readme.md".to_string()),
        "{docs:?}"
    );
    assert!(
        docs.contains(&"docs/slice-1-review-1.md".to_string()),
        "{docs:?}"
    );
}

#[test]
fn relative_links_in_the_docs_resolve() {
    let crate_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    for doc in docs() {
        let path = crate_root.join(&doc);
        let text = fs::read_to_string(&path).unwrap();
        let folder = path.parent().unwrap();
        for link in relative_links(&text) {
            assert!(
                folder.join(&link).exists(),
                "{doc} links to {link}, which doesn't exist"
            );
        }
    }
}

#[test]
fn line_anchors_name_lines_that_exist() {
    // Only that the lines exist: whether they still hold what the doc
    // means can't be checked. Reviews name the commit their lines point at.
    let crate_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut checked = 0;
    for doc in docs() {
        let path = crate_root.join(&doc);
        let text = fs::read_to_string(&path).unwrap();
        let folder = path.parent().unwrap();
        for (link, anchor) in relative_links_with_anchors(&text) {
            let Some(anchor) = anchor else { continue };
            // GitHub's heading anchors are lower case, so `L` means lines.
            if !anchor.starts_with('L') {
                continue;
            }
            let Some((first, last)) = line_range(&anchor) else {
                panic!("{doc} links to {link}#{anchor}, which isn't a line anchor");
            };
            let lines = fs::read_to_string(folder.join(&link))
                .unwrap()
                .lines()
                .count();
            assert!(
                1 <= first && first <= last && last <= lines,
                "{doc} links to {link}#{anchor}, but {link} has {lines} lines"
            );
            checked += 1;
        }
    }
    assert!(checked > 0, "no line anchors were found to check");
}

#[test]
fn line_ranges_are_read_from_anchors() {
    assert_eq!(line_range("L12"), Some((12, 12)));
    assert_eq!(line_range("L12-L30"), Some((12, 30)));
    assert_eq!(line_range("Lx"), None);
    assert_eq!(line_range("json-output"), None);
}
