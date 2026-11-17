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

/// The same links, each with its `#section` part, if it has one. A link to
/// a heading in the same doc, such as `#fixes-after-review-1`, has the path
/// `""`. Links in fenced code blocks and code spans are examples, not links,
/// so they're skipped.
fn relative_links_with_anchors(markdown: &str) -> Vec<(String, Option<String>)> {
    let mut links = Vec::new();
    for line in prose_lines(markdown) {
        let line = without_code_spans(line);
        let mut rest = line.as_str();
        while let Some(start) = rest.find("](") {
            rest = &rest[start + 2..];
            let Some(end) = rest.find(')') else {
                break;
            };
            let target = &rest[..end];
            rest = &rest[end..];
            if target.contains("://") || target.starts_with("mailto:") {
                continue;
            }
            let (path, anchor) = match target.split_once('#') {
                Some((path, anchor)) => (path, Some(anchor.to_string())),
                None => (target, None),
            };
            links.push((path.to_string(), anchor));
        }
    }
    links
}

/// `line` with each code span, such as `` `[a](b.md)` ``, taken out. A
/// span opened with a run of backticks is closed by a run of the same
/// length, and an unclosed backtick is just a backtick.
fn without_code_spans(line: &str) -> String {
    let mut out = String::new();
    let mut rest = line;
    while let Some(start) = rest.find('`') {
        out.push_str(&rest[..start]);
        let after = &rest[start..];
        let ticks = after.chars().take_while(|c| *c == '`').count();
        match after[ticks..].find(&after[..ticks]) {
            Some(end) => rest = &after[ticks + end + ticks..],
            None => {
                out.push_str(after);
                rest = "";
            }
        }
    }
    out.push_str(rest);
    out
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
    let mut failures = Vec::new();
    for doc in docs() {
        let text = fs::read_to_string(crate_root.join(&doc)).unwrap();
        let at = review_checked_at(crate_root, &doc);
        failures.extend(path_failures(crate_root, &doc, &text, at.as_deref()));
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

/// A message for each link in `text`, the doc at `doc` relative to `root`,
/// whose path doesn't exist: at commit `at` if it's given
/// (`review_checked_at`), or as the files are now.
fn path_failures(root: &Path, doc: &str, text: &str, at: Option<&str>) -> Vec<String> {
    let mut failures = Vec::new();
    for link in relative_links(text) {
        let exists = match at {
            Some(commit) => {
                let file = file_in_root(doc, &link);
                git_succeeds(root, &["cat-file", "-e", &format!("{commit}:./{file}")])
            }
            None => root.join(doc).parent().unwrap().join(&link).exists(),
        };
        if !exists {
            let when = at.map(|commit| format!(" at {commit}")).unwrap_or_default();
            failures.push(format!("{doc} links to {link}, which doesn't exist{when}"));
        }
    }
    failures
}

/// When a review's links are checked: as of the last commit that changed
/// it, so a review stays a record of what was true when it was written,
/// and a later change to DESIGN.md or a readme can't break it. `None`, for
/// "as the files are now", for a doc that isn't a review, and for a review
/// that's untracked or has uncommitted changes, since its links have to
/// work once it's committed.
fn review_checked_at(root: &Path, doc: &str) -> Option<String> {
    if doc.starts_with("docs/reviews/") {
        checked_at(root, doc)
    } else {
        None
    }
}

/// The last commit that changed `doc`, relative to `root`, or `None` if
/// it's untracked or has uncommitted changes, or git can't say.
fn checked_at(root: &Path, doc: &str) -> Option<String> {
    let status = git_output(root, &["status", "--porcelain", "--", doc])?;
    if !status.trim().is_empty() {
        return None;
    }
    let commit = git_output(root, &["log", "-1", "--format=%h", "--", doc])?;
    let commit = commit.trim();
    (!commit.is_empty()).then(|| commit.to_string())
}

/// What `git <args>` prints in `root`, or `None` if it fails.
fn git_output(root: &Path, args: &[&str]) -> Option<String> {
    let output = std::process::Command::new("git")
        .args(args)
        .current_dir(root)
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    String::from_utf8(output.stdout).ok()
}

/// Whether `git <args>` succeeds in `root`.
fn git_succeeds(root: &Path, args: &[&str]) -> bool {
    git_output(root, args).is_some()
}

/// The file `link` names, from the doc at `doc`, as a path relative to the
/// root with `..` worked out, such as `docs/DESIGN.md` for `../DESIGN.md`
/// from `docs/reviews/r.md`. Worked out from the text, not the disk, since
/// the file may not exist now.
fn file_in_root(doc: &str, link: &str) -> String {
    let mut parts: Vec<&str> = doc.split('/').collect();
    parts.pop();
    for part in link.split('/') {
        match part {
            "" | "." => {}
            ".." => {
                parts.pop();
            }
            other => parts.push(other),
        }
    }
    parts.join("/")
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

/// The text of `file`, relative to the crate root, at `commit`.
fn text_at(commit: &str, file: &str) -> String {
    let crate_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    match try_text_at(crate_root, commit, file) {
        Ok(text) => text,
        Err(e) => panic!("can't read {file} at {commit}: {e}"),
    }
}

/// The text of `file`, relative to `root`, at `commit` in the repository
/// `root` is in, or git's message if it can't be read there, such as when
/// the file didn't exist yet.
fn try_text_at(root: &Path, commit: &str, file: &str) -> Result<String, String> {
    let output = std::process::Command::new("git")
        .args(["show", &format!("{commit}:./{file}")])
        .current_dir(root)
        .output()
        .map_err(|e| format!("can't run git: {e}"))?;
    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).trim().to_string());
    }
    String::from_utf8(output.stdout).map_err(|e| e.to_string())
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
            // An empty path is a link into the doc itself.
            let target = if link.is_empty() {
                fs::canonicalize(&path).unwrap()
            } else {
                fs::canonicalize(folder.join(&link)).unwrap()
            };
            let lines = match &commit {
                Some(commit) if is_review => {
                    let file = target.strip_prefix(&canonical_root).unwrap();
                    text_at(commit, file.to_str().unwrap()).lines().count()
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
fn heading_anchors_name_headings_that_exist() {
    let crate_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut checked = 0;
    let mut failures = Vec::new();
    for doc in docs() {
        let text = fs::read_to_string(crate_root.join(&doc)).unwrap();
        let at = review_checked_at(crate_root, &doc);
        let (count, found) = heading_link_failures(crate_root, &doc, &text, at.as_deref());
        checked += count;
        failures.extend(found);
    }
    assert!(checked > 0, "no heading anchors were found to check");
    assert!(failures.is_empty(), "{}", failures.join("\n"));
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

/// `line` without up to three leading spaces, or `None` if it's indented
/// four or more, which makes it code rather than a heading or a fence.
fn strip_indent(line: &str) -> Option<&str> {
    let trimmed = line.trim_start_matches(' ');
    (line.len() - trimmed.len() <= 3).then_some(trimmed)
}

/// The fence a line opens or closes a fenced code block with, if any.
fn fence_marker(line: &str) -> Option<&'static str> {
    let rest = strip_indent(line)?;
    ["```", "~~~"]
        .into_iter()
        .find(|marker| rest.starts_with(marker))
}

/// The lines of `markdown` outside fenced code blocks. A block opened with
/// ``` is closed only by ```, and one opened with ~~~ only by ~~~.
fn prose_lines(markdown: &str) -> Vec<&str> {
    let mut lines = Vec::new();
    let mut fence: Option<&str> = None;
    for line in markdown.lines() {
        if let Some(marker) = fence_marker(line) {
            match fence {
                None => fence = Some(marker),
                Some(open) if open == marker => fence = None,
                Some(_) => {}
            }
            continue;
        }
        if fence.is_none() {
            lines.push(line);
        }
    }
    lines
}

/// A heading's text, if `line` is an ATX heading: one to six `#`s, then a
/// space, without any closing `#`s.
fn heading_text(line: &str) -> Option<&str> {
    let rest = strip_indent(line)?;
    let hashes = rest.chars().take_while(|c| *c == '#').count();
    if !(1..=6).contains(&hashes) {
        return None;
    }
    let rest = &rest[hashes..];
    if !rest.starts_with([' ', '\t']) {
        return None;
    }
    let text = rest.trim();
    // A closing run of `#`s counts only after a space, or as the whole text.
    let without = text.trim_end_matches('#');
    if without.is_empty() || without.ends_with([' ', '\t']) {
        return Some(without.trim_end());
    }
    Some(text)
}

/// The text GitHub renders for a heading: code spans become their contents,
/// and the rest loses its markup (`markup_free`).
fn rendered_text(heading: &str) -> String {
    let mut out = String::new();
    let mut rest = heading;
    while let Some(start) = rest.find('`') {
        out.push_str(&markup_free(&rest[..start]));
        let after = &rest[start..];
        let ticks = after.chars().take_while(|c| *c == '`').count();
        let body = &after[ticks..];
        match body.find(&after[..ticks]) {
            Some(end) => {
                out.push_str(&body[..end]);
                rest = &body[end + ticks..];
            }
            // An unclosed backtick is just a backtick.
            None => {
                out.push_str(after);
                rest = "";
            }
        }
    }
    out.push_str(&markup_free(rest));
    out
}

/// Text outside code spans without its markup: `[text](url)` becomes
/// `text`, HTML tags go, and so do the `_`s of emphasis, as in
/// `_Done when_` (`emphasis_underscores`). Any other `_`, as in
/// `snake_case` or `the _id field`, stays. `*` needs no handling, since
/// slugging drops it anyway.
fn markup_free(text: &str) -> String {
    let chars: Vec<char> = text.chars().collect();
    let emphasis = emphasis_underscores(&chars);
    let mut out = String::new();
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        if c == '<'
            && let Some(len) = chars[i..].iter().position(|c| *c == '>')
        {
            i += len + 1;
            continue;
        }
        if c == ']' && chars.get(i + 1) == Some(&'(') {
            // The end of a link's text: skip its target.
            match chars[i..].iter().position(|c| *c == ')') {
                Some(len) => i += len + 1,
                None => i += 1,
            }
            continue;
        }
        if emphasis[i] {
            i += 1;
            continue;
        }
        // A link's `[` and an image's `!` stay, since slugging drops them.
        out.push(c);
        i += 1;
    }
    out
}

/// Which of `chars` are the underscores of emphasis. A run of `_`s that can
/// open (not after a letter or digit, and not before a space or the end)
/// pairs with the next run of the same length that can close (not after a
/// space, and not before a letter or digit), and both go. A run with no
/// partner, such as the one in `the _id field`, is literal, as it is on
/// GitHub, and so is a run inside a word, as in `snake_case`.
fn emphasis_underscores(chars: &[char]) -> Vec<bool> {
    // Each run of `_`s: where it starts, how long it is, whether it can
    // open and whether it can close.
    let mut runs = Vec::new();
    let mut i = 0;
    while i < chars.len() {
        if chars[i] != '_' {
            i += 1;
            continue;
        }
        let len = chars[i..].iter().take_while(|c| **c == '_').count();
        let before = i.checked_sub(1).map(|j| chars[j]);
        let after = chars.get(i + len).copied();
        let opens =
            !before.is_some_and(char::is_alphanumeric) && after.is_some_and(|c| !c.is_whitespace());
        let closes =
            before.is_some_and(|c| !c.is_whitespace()) && !after.is_some_and(char::is_alphanumeric);
        runs.push((i, len, opens, closes));
        i += len;
    }

    let mut emphasis = vec![false; chars.len()];
    let mut used = vec![false; runs.len()];
    for open in 0..runs.len() {
        let (start, len, opens, _) = runs[open];
        if used[open] || !opens {
            continue;
        }
        let partner = (open + 1..runs.len())
            .find(|&close| !used[close] && runs[close].1 == len && runs[close].3);
        if let Some(close) = partner {
            used[open] = true;
            used[close] = true;
            for run in [start, runs[close].0] {
                emphasis[run..run + len].fill(true);
            }
        }
    }
    emphasis
}

/// GitHub's slug for a heading's text: lower case, keeping letters, digits,
/// spaces, `-` and `_`, with each space turned into `-`. GitHub also keeps
/// combining marks, which `char::is_alphanumeric` doesn't, but no doc has a
/// heading with one.
fn slug(text: &str) -> String {
    text.trim()
        .to_lowercase()
        .chars()
        .filter(|c| c.is_alphanumeric() || matches!(c, ' ' | '-' | '_'))
        .map(|c| if c == ' ' { '-' } else { c })
        .collect()
}

/// The anchors GitHub gives a markdown file's headings, in order. A heading
/// whose slug is already taken gets `-1`, then `-2` and so on.
fn heading_anchors(markdown: &str) -> Vec<String> {
    let mut seen: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
    let mut anchors = Vec::new();
    for line in prose_lines(markdown) {
        let Some(text) = heading_text(line) else {
            continue;
        };
        let base = slug(&rendered_text(text));
        let count = seen.entry(base.clone()).or_insert(0);
        anchors.push(if *count == 0 {
            base
        } else {
            format!("{base}-{count}")
        });
        *count += 1;
    }
    anchors
}

#[test]
fn heading_anchors_are_slugged_like_github() {
    let doc = "\
# Design Document: `tracker`, a verification-gated work tracker
## 3. Interface
#### 2.3.1 Judges (Claude via the `claude` CLI)
## What's in this slice
## Limits, and what later slices change
### Low severity
### Low severity
```markdown
## Work tracking (`tracker`)
```
~~~
# Not a heading
```
still in the tilde fence
~~~
## _Done when_ and **bold** and *star* [linked](PLAN.md) ##
## `TRACKER_HOME` and snake_case_word
#hashtag, not a heading
    ## Indented four spaces is code
   ### Indented three spaces is a heading
## Ünïcode “quotes” — and é
### 2. A planted `<id>.json.tmp` symlink
### A <b>bold</b> tag
## The _id field
## _foo_bar_ and __init__ and a_b
";
    assert_eq!(
        heading_anchors(doc),
        [
            "design-document-tracker-a-verification-gated-work-tracker",
            "3-interface",
            "231-judges-claude-via-the-claude-cli",
            "whats-in-this-slice",
            "limits-and-what-later-slices-change",
            "low-severity",
            "low-severity-1",
            "done-when-and-bold-and-star-linked",
            "tracker_home-and-snake_case_word",
            "indented-three-spaces-is-a-heading",
            "ünïcode-quotes--and-é",
            // In a code span `<id>` is text, but outside one a tag is markup.
            "2-a-planted-idjsontmp-symlink",
            "a-bold-tag",
            // Only `_`s that pair up as emphasis go.
            "the-_id-field",
            "foo_bar-and-init-and-a_b",
        ]
    );
}

#[test]
fn links_in_code_are_skipped_and_same_doc_links_are_kept() {
    let doc = "\
See [§3](DESIGN.md#3-interface) and [below](#fixes-after-review-1).
Not `[a span](span.md)` or ``[two `ticks`](ticks.md)``, but [after](after.md).
[site](https://example.com) and [mail](mailto:me@example.com)
```markdown
[fenced](fenced.md)
```
[last](PLAN.md)
";
    let links = relative_links_with_anchors(doc);
    let expected = [
        ("DESIGN.md", Some("3-interface")),
        ("", Some("fixes-after-review-1")),
        ("after.md", None),
        ("PLAN.md", None),
    ]
    .map(|(path, anchor)| (path.to_string(), anchor.map(String::from)));
    assert_eq!(links, expected);
}

/// Checks the heading anchors of the links in `text`, the doc at `doc`
/// relative to `root`, and returns how many it checked and a message for
/// each one that fails. An anchor must match a heading in its target,
/// slugged the way GitHub does it, at commit `at` if it's given
/// (`review_checked_at`), or as the target is now. A link into the doc
/// itself is checked against `text`. `L` anchors are left to
/// `line_anchors_name_lines_that_exist`, since GitHub's heading anchors are
/// lower case.
fn heading_link_failures(
    root: &Path,
    doc: &str,
    text: &str,
    at: Option<&str>,
) -> (usize, Vec<String>) {
    let folder = root.join(doc).parent().unwrap().to_path_buf();
    let when = at.map(|commit| format!(" at {commit}")).unwrap_or_default();
    let mut checked = 0;
    let mut failures = Vec::new();
    for (link, anchor) in relative_links_with_anchors(text) {
        let Some(anchor) = anchor else { continue };
        if anchor.starts_with('L') {
            continue;
        }
        let shown = format!("{link}#{anchor}");
        let name = if link.is_empty() { doc } else { link.as_str() };
        if !name.ends_with(".md") {
            failures.push(format!(
                "{doc} links to {shown}, but #{anchor} isn't a line anchor, and {name} isn't \
                 Markdown"
            ));
            continue;
        }
        checked += 1;
        let target = if link.is_empty() {
            Ok(text.to_string())
        } else {
            match at {
                Some(commit) => try_text_at(root, commit, &file_in_root(doc, &link)),
                None => fs::read_to_string(folder.join(&link)).map_err(|e| e.to_string()),
            }
        };
        match target {
            Err(e) => failures.push(format!(
                "{doc} links to {shown}, but {name} can't be read{when}: {e}"
            )),
            Ok(target) if !heading_anchors(&target).contains(&anchor) => failures.push(format!(
                "{doc} links to {shown}, but {name} has no heading with the anchor \
                     #{anchor}{when}"
            )),
            Ok(_) => {}
        }
    }
    (checked, failures)
}

/// Runs git in `dir`, with an identity of its own so the test doesn't need
/// one set up.
fn git(dir: &Path, args: &[&str]) {
    let output = std::process::Command::new("git")
        .args([
            "-c",
            "user.name=test",
            "-c",
            "user.email=test@example.invalid",
        ])
        .args(args)
        .current_dir(dir)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "git {args:?}: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn a_review_is_checked_as_of_the_last_commit_that_changed_it() {
    // In a throwaway repo: a review links to DESIGN.md's `## Old name`, and
    // both are committed. Then DESIGN.md renames the heading.
    let repo = tempfile::TempDir::new().unwrap();
    let root = repo.path();
    fs::create_dir_all(root.join("docs/reviews")).unwrap();
    fs::write(
        root.join("docs/DESIGN.md"),
        "# Design\n## Old name\nText.\n",
    )
    .unwrap();
    let review = "docs/reviews/r.md";
    let text = "## Findings\nSee [then](../DESIGN.md#old-name) and [here](#findings).\n";
    fs::write(root.join(review), text).unwrap();
    git(root, &["init", "-q"]);
    git(root, &["add", "."]);
    git(root, &["commit", "-q", "-m", "docs: design and review"]);
    let written = head(root);
    fs::write(
        root.join("docs/DESIGN.md"),
        "# Design\n## New name\nText.\n",
    )
    .unwrap();
    git(root, &["commit", "-q", "-am", "docs: rename the heading"]);

    // The review hasn't changed since, so the rename doesn't break it.
    assert_eq!(checked_at(root, review), Some(written.clone()));
    let (checked, failures) = heading_link_failures(root, review, text, Some(&written));
    assert_eq!(checked, 2);
    assert!(failures.is_empty(), "{failures:?}");
    assert!(path_failures(root, review, text, Some(&written)).is_empty());

    // Once the review is edited, it's checked as it is now, so the link
    // has to be fixed along with whatever else changed.
    let edited = format!("{text}An outcome, added later.\n");
    fs::write(root.join(review), &edited).unwrap();
    assert_eq!(checked_at(root, review), None);
    let (_, failures) = heading_link_failures(root, review, &edited, None);
    assert_eq!(failures.len(), 1, "{failures:?}");
    assert!(failures[0].contains(review), "{failures:?}");
    assert!(failures[0].contains("#old-name"), "{failures:?}");

    // Committed with the link fixed, it's checked at that commit.
    let fixed = edited.replace("#old-name", "#new-name");
    fs::write(root.join(review), &fixed).unwrap();
    git(root, &["commit", "-q", "-am", "docs: record the outcome"]);
    let at = checked_at(root, review).unwrap();
    assert_ne!(at, written);
    let (_, failures) = heading_link_failures(root, review, &fixed, Some(&at));
    assert!(failures.is_empty(), "{failures:?}");

    // A link that was already broken when the review was committed fails,
    // naming the commit, even if the heading exists now.
    let early = "See [later](../DESIGN.md#new-name) and [gone](../gone.md).\n";
    let (_, failures) = heading_link_failures(root, review, early, Some(&written));
    assert_eq!(failures.len(), 1, "{failures:?}");
    assert!(failures[0].contains(&written), "{failures:?}");
    let failures = path_failures(root, review, early, Some(&written));
    assert_eq!(failures.len(), 1, "{failures:?}");
    assert!(failures[0].contains("../gone.md"), "{failures:?}");

    // An untracked doc is checked as it is now.
    fs::write(root.join("docs/reviews/new.md"), "x\n").unwrap();
    assert_eq!(checked_at(root, "docs/reviews/new.md"), None);
}

/// The short ID of `HEAD` in the repository at `dir`.
fn head(dir: &Path) -> String {
    let output = std::process::Command::new("git")
        .args(["rev-parse", "--short", "HEAD"])
        .current_dir(dir)
        .output()
        .unwrap();
    String::from_utf8(output.stdout).unwrap().trim().to_string()
}

#[test]
fn an_unreadable_target_is_a_failure_that_names_the_link() {
    let dir = tempfile::TempDir::new().unwrap();
    let root = dir.path();
    fs::create_dir_all(root.join("docs/folder.md")).unwrap();
    let text = "[gone](missing.md#nowhere) and [folder](folder.md#x) and [web](page.html#top)\n";
    let doc = "docs/readme.md";
    fs::write(root.join(doc), text).unwrap();

    let (_, failures) = heading_link_failures(root, doc, text, None);
    assert_eq!(failures.len(), 3, "{failures:?}");
    for (failure, link) in
        failures
            .iter()
            .zip(["missing.md#nowhere", "folder.md#x", "page.html#top"])
    {
        assert!(failure.starts_with(doc), "{failure}");
        assert!(failure.contains(link), "{failure}");
    }
    assert!(failures[2].contains("isn't Markdown"), "{failures:?}");
}
