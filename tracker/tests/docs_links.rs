//! DESIGN.md, PLAN.md, walkthrough.html and judge-eval/ moved into this
//! folder together in slice 1. Their relative links must still resolve.

use std::fs;
use std::path::Path;

/// The relative targets of a markdown file's `[text](target)` links, without
/// any `#section` part.
fn relative_links(markdown: &str) -> Vec<String> {
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
        let path = target.split('#').next().unwrap_or_default();
        links.push(path.to_string());
    }
    links
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
