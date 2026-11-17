//! DESIGN.md, PLAN.md, walkthrough.html and judge-eval/ moved into this
//! folder together in slice 1, and DESIGN.md and PLAN.md later into `docs/`.
//! Their relative links must still resolve.

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
    let design = fs::read_to_string(crate_root.join("docs/DESIGN.md")).unwrap();
    let links = relative_links(&design);
    for expected in ["../judge-eval/", "PLAN.md", "../walkthrough.html"] {
        assert!(
            links.iter().any(|link| link == expected),
            "DESIGN.md no longer links to {expected}"
        );
    }
}

/// The docs whose links are checked: CLAUDE.md, judge-eval's README, and
/// every markdown file in `docs/` and the folders under it, DESIGN.md, PLAN.md
/// and the reviews included, so a new doc there is checked without being
/// listed here.
fn docs() -> Vec<String> {
    let mut docs: Vec<String> = ["CLAUDE.md", "judge-eval/README.md"]
        .map(String::from)
        .to_vec();
    let mut in_docs = Vec::new();
    markdown_in("docs", &mut in_docs);
    in_docs.sort();
    docs.extend(in_docs);
    docs
}

/// Adds every markdown file in `folder`, relative to the crate root, and in
/// the folders under it, to `found`.
fn markdown_in(folder: &str, found: &mut Vec<String>) {
    let crate_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    for entry in fs::read_dir(crate_root.join(folder)).unwrap() {
        let entry = entry.unwrap();
        let name = format!("{folder}/{}", entry.file_name().into_string().unwrap());
        if entry.file_type().unwrap().is_dir() {
            markdown_in(&name, found);
        } else if name.ends_with(".md") {
            found.push(name);
        }
    }
}

#[test]
fn the_docs_folder_is_checked() {
    let docs = docs();
    for doc in [
        "docs/DESIGN.md",
        "docs/PLAN.md",
        "docs/slice-1-readme.md",
        "docs/reviews/slice-1-review-1.md",
    ] {
        assert!(docs.contains(&doc.to_string()), "{doc} isn't in {docs:?}");
    }
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

/// The commit a review's line links point at, from its `| Line references |`
/// row, such as `54d8f66` from "Point at `54d8f66`". `None` for a doc with no
/// such row, or one whose row names no commit, such as slice 1 review 1's
/// "the uncommitted working tree".
fn pinned_commit(markdown: &str) -> Option<String> {
    let row = markdown
        .lines()
        .find(|line| line.starts_with("| Line references"))?;
    let (_, rest) = row.split_once('`')?;
    let (commit, _) = rest.split_once('`')?;
    let is_sha = commit.len() >= 7 && commit.chars().all(|c| c.is_ascii_hexdigit());
    is_sha.then(|| commit.to_string())
}

/// How many lines `file`, relative to the crate root, has at `commit`.
fn lines_at(commit: &str, file: &str) -> usize {
    let crate_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let output = std::process::Command::new("git")
        .args(["show", &format!("{commit}:./{file}")])
        .current_dir(crate_root)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "can't read {file} at {commit}: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout).unwrap().lines().count()
}

#[test]
fn line_anchors_name_lines_that_exist() {
    // Only that the lines exist: whether they still hold what the doc
    // means can't be checked. A review's line links point at the commit
    // it names, so they're checked against the files at that commit,
    // and later changes to the code can't break them. A review that names
    // no commit can't be checked.
    let crate_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let canonical_root = fs::canonicalize(crate_root).unwrap();
    let mut checked = 0;
    for doc in docs() {
        let path = crate_root.join(&doc);
        let text = fs::read_to_string(&path).unwrap();
        let folder = path.parent().unwrap();
        let is_review = doc.starts_with("docs/reviews/");
        let commit = pinned_commit(&text);
        if is_review && commit.is_none() {
            continue;
        }
        for (link, anchor) in relative_links_with_anchors(&text) {
            let Some(anchor) = anchor else { continue };
            // GitHub's heading anchors are lower case, so `L` means lines.
            if !anchor.starts_with('L') {
                continue;
            }
            let Some((first, last)) = line_range(&anchor) else {
                panic!("{doc} links to {link}#{anchor}, which isn't a line anchor");
            };
            let target = fs::canonicalize(folder.join(&link)).unwrap();
            let lines = match &commit {
                Some(commit) if is_review => {
                    let file = target.strip_prefix(&canonical_root).unwrap();
                    lines_at(commit, file.to_str().unwrap())
                }
                _ => fs::read_to_string(&target).unwrap().lines().count(),
            };
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
fn a_review_names_the_commit_its_lines_point_at() {
    assert_eq!(
        pinned_commit("| Date | x |\n| Line references | Point at `54d8f66` |\n"),
        Some("54d8f66".to_string())
    );
    assert_eq!(
        pinned_commit("| Line references | Point at the uncommitted working tree |\n"),
        None
    );
    assert_eq!(pinned_commit("# Slice 2\n"), None);
    let review = fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("docs/reviews/slice-2-review-1.md"),
    )
    .unwrap();
    assert_eq!(pinned_commit(&review), Some("54d8f66".to_string()));
}

#[test]
fn line_ranges_are_read_from_anchors() {
    assert_eq!(line_range("L12"), Some((12, 12)));
    assert_eq!(line_range("L12-L30"), Some((12, 30)));
    assert_eq!(line_range("Lx"), None);
    assert_eq!(line_range("json-output"), None);
}
