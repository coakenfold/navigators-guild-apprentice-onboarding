//! What the gates read from git (§2.1, G5, §2.7): `HEAD`, whether the tree
//! under a project's root is clean, an item's commits, and the ref that
//! keeps a verified commit. `git` runs as a subprocess in the project's
//! root, which can be a folder inside a larger repository.
//!
//! Every function here fails with exit 3 when git can't answer, since then
//! nothing can be proved against a commit (§3, §4.4).

use std::path::Path;
use std::process::{Command, Stdio};

use crate::error::{Error, Result};

/// How many uncommitted files G5 names before it says how many more there
/// are.
pub const LISTED_FILES: usize = 10;

/// Runs `git <args>` in `root` and returns its stdout, or fails with exit 3
/// saying what git said. Settings that would change what git prints for a
/// command that reads it, such as signatures in `log`, are turned off.
fn git(root: &Path, args: &[&str]) -> Result<String> {
    let output = Command::new("git")
        .current_dir(root)
        .args([
            "-c",
            "log.showSignature=false",
            "-c",
            "core.quotePath=false",
        ])
        .args(args)
        .stdin(Stdio::null())
        .output()
        .map_err(|e| {
            Error::store(format!(
                "Can't run git in {}: {e}. tracker needs git installed to prove work against a \
                 commit.",
                root.display()
            ))
        })?;
    if !output.status.success() {
        let said = String::from_utf8_lossy(&output.stderr);
        return Err(Error::store(format!(
            "`git {}` failed in {}: {}",
            args.join(" "),
            root.display(),
            said.trim()
        )));
    }
    String::from_utf8(output.stdout).map_err(|_| {
        Error::store(format!(
            "`git {}` in {} printed something that isn't UTF-8.",
            args.join(" "),
            root.display()
        ))
    })
}

/// Refuses, with exit 3, a project root that isn't inside a git work tree
/// (§4.4). `key` names the project, and `id` the item, in the message.
pub fn require_work_tree(root: &Path, key: &str, id: &str) -> Result<()> {
    if is_work_tree(root)? {
        return Ok(());
    }
    Err(Error::store(format!(
        "{key}'s root, {}, isn't inside a git work tree, so tracker can't record or prove \
         {id}'s work against a commit. Make it a repository with `git init`, or register \
         {key} with a root that's inside one. Nothing was changed.",
        root.display()
    )))
}

/// Whether `root` is inside a git work tree. Exits 3 if git can't be run.
pub fn is_work_tree(root: &Path) -> Result<bool> {
    let inside = Command::new("git")
        .current_dir(root)
        .args(["rev-parse", "--is-inside-work-tree"])
        .stdin(Stdio::null())
        .stderr(Stdio::null())
        .output();
    match inside {
        Ok(output) => Ok(output.status.success() && output.stdout.starts_with(b"true")),
        Err(e) => Err(Error::store(format!(
            "Can't run git in {}: {e}. tracker needs git installed to prove work against a \
             commit. Nothing was changed.",
            root.display()
        ))),
    }
}

/// The commit `HEAD` points to, as a full SHA, or `None` in a repository
/// with no commits yet.
pub fn head(root: &Path) -> Result<Option<String>> {
    let output = Command::new("git")
        .current_dir(root)
        .args(["rev-parse", "--verify", "--quiet", "HEAD^{commit}"])
        .stdin(Stdio::null())
        .stderr(Stdio::null())
        .output()
        .map_err(|e| Error::store(format!("Can't run git in {}: {e}.", root.display())))?;
    if !output.status.success() {
        return Ok(None);
    }
    let sha = String::from_utf8_lossy(&output.stdout).trim().to_string();
    Ok((!sha.is_empty()).then_some(sha))
}

/// Every file under `root` with uncommitted changes, staged or not, or
/// untracked, by its path from the top of the repository, each with what's
/// wrong with it, such as `modified xx/a.txt`. Files that git ignores,
/// whether by `.gitignore`, `.git/info/exclude` or `core.excludesFile`, and
/// files outside the root, aren't listed (G5, §2.1).
pub fn uncommitted(root: &Path) -> Result<Vec<String>> {
    // `-z` keeps names exactly as they are, and `--untracked-files=all`
    // names each untracked file, whatever the user's settings say.
    let text = git(
        root,
        &[
            "status",
            "--porcelain=v1",
            "-z",
            "--untracked-files=all",
            "--",
            ".",
        ],
    )?;
    let mut files = Vec::new();
    let mut entries = text.split('\0');
    while let Some(entry) = entries.next() {
        if entry.len() < 4 {
            continue;
        }
        let (code, path) = entry.split_at(3);
        let code = code.trim();
        let what = match code {
            "??" => "untracked",
            _ if code.contains('D') => "deleted",
            _ if code.contains('A') => "added",
            _ if code.contains('R') => "renamed",
            _ if code.contains('C') => "copied",
            _ if code.contains('U') => "unmerged",
            _ => "modified",
        };
        // A rename or copy is followed by the path it came from.
        if code.contains('R') || code.contains('C') {
            entries.next();
        }
        files.push(format!("{what} {path}"));
    }
    Ok(files)
}

/// Every tracked file under `root` that git is told not to look at, so that
/// `git status` can't see a change to it, by its path from the top of the
/// repository, each with its flag, such as `skip-worktree xx/a.txt` (G5,
/// §2.4). A file is listed whether or not it's on disk, since `git status`
/// doesn't report a flagged file's deletion either. The one exception is a
/// skip-worktree file that isn't on disk in a sparse checkout
/// (`core.sparseCheckout` true), where that's how files are left out.
pub fn hidden(root: &Path) -> Result<Vec<String>> {
    // `ls-files -v` tags a skip-worktree file `S` and an assume-unchanged
    // one in lower case. Its paths are from `root`, and the prefix makes
    // them from the top, as `uncommitted` names them.
    let prefix = git(root, &["rev-parse", "--show-prefix"])?;
    let text = git(root, &["ls-files", "-v", "-z", "--", "."])?;
    let sparse = git(
        root,
        &[
            "config",
            "--bool",
            "--default",
            "false",
            "core.sparseCheckout",
        ],
    )?;
    let sparse = sparse.trim() == "true";
    let mut files = Vec::new();
    for entry in text.split('\0') {
        let Some((tag, path)) = entry.split_at_checked(2) else {
            continue;
        };
        let what = match tag.trim() {
            "S" | "s" => "skip-worktree",
            tag if tag.chars().any(|c| c.is_ascii_lowercase()) => "assume-unchanged",
            _ => continue,
        };
        let left_out = sparse && what == "skip-worktree";
        if left_out && root.join(path).symlink_metadata().is_err() {
            continue;
        }
        files.push(format!("{what} {}{path}", prefix.trim_end()));
    }
    Ok(files)
}

/// Whether `header`, a commit message's first line, is scoped to item `id`:
/// `^[a-z]+\(<id>\)!?: ` (§2.1). So `feat(BM-1): …` and `fix(BM-1)!: …` are
/// BM-1's, and `feat(BM-12): …` and `BM-1: …` aren't.
pub fn is_item_header(header: &str, id: &str) -> bool {
    let kind = header.bytes().take_while(u8::is_ascii_lowercase).count();
    if kind == 0 {
        return false;
    }
    let Some(rest) = header[kind..]
        .strip_prefix('(')
        .and_then(|rest| rest.strip_prefix(id))
        .and_then(|rest| rest.strip_prefix(')'))
    else {
        return false;
    };
    let rest = rest.strip_prefix('!').unwrap_or(rest);
    rest.starts_with(": ")
}

/// Item `id`'s commits (§2.1): the SHAs of the commits reachable from
/// `HEAD` whose header is scoped to it, newest first. None in a repository
/// with no commits yet.
pub fn item_commits(root: &Path, id: &str) -> Result<Vec<String>> {
    let commits = item_log(root, id)?;
    Ok(commits.into_iter().map(|(sha, _)| sha).collect())
}

/// Item `id`'s commits, as `item_commits` finds them, each with its
/// header: the first line of its message.
pub fn item_log(root: &Path, id: &str) -> Result<Vec<(String, String)>> {
    if head(root)?.is_none() {
        return Ok(Vec::new());
    }
    // One record per commit, ended by a NUL: its SHA, then its message.
    let text = git(root, &["log", "-z", "--format=%H%n%B", "HEAD"])?;
    let commits = text
        .split('\0')
        .filter_map(|record| {
            let mut lines = record.trim_start_matches('\n').lines();
            let sha = lines.next()?;
            let header = lines.next().unwrap_or_default();
            is_item_header(header, id).then(|| (sha.to_string(), header.to_string()))
        })
        .collect();
    Ok(commits)
}

/// Each commit in `shas` with its patch, in the order given, as `git show`
/// prints them, for `show --diff`. Every file a commit touched is shown,
/// inside the project's root or not (§2.1). Bytes that aren't UTF-8, as in
/// a file in another encoding, are shown as `�` rather than refused, since
/// this is only for reading. External diff drivers and textconv filters,
/// which `.git/config` can set without any commit showing it, are turned
/// off, and so is `diff.relative`, so the diff is what the commits hold.
pub fn patches(root: &Path, shas: &[String]) -> Result<String> {
    if shas.is_empty() {
        return Ok(String::new());
    }
    let output = Command::new("git")
        .current_dir(root)
        .args([
            "-c",
            "log.showSignature=false",
            "-c",
            "core.quotePath=false",
            "show",
            "--no-color",
            "--no-ext-diff",
            "--no-textconv",
            "--no-relative",
            "--patch",
            "--format=commit %H%n%s",
        ])
        .args(shas)
        .stdin(Stdio::null())
        .output()
        .map_err(|e| Error::store(format!("Can't run git in {}: {e}.", root.display())))?;
    if !output.status.success() {
        return Err(Error::store(format!(
            "`git show` failed in {}: {}",
            root.display(),
            String::from_utf8_lossy(&output.stderr).trim()
        )));
    }
    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}

/// The text of `path` (from the top of the repository) as commit `rev`
/// holds it, or `None` if it has none there or isn't UTF-8.
pub fn file_at(root: &Path, rev: &str, path: &str) -> Option<String> {
    let output = Command::new("git")
        .current_dir(root)
        .args(["show", "--no-textconv", &format!("{rev}:{path}")])
        .stdin(Stdio::null())
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    String::from_utf8(output.stdout).ok()
}

/// The project root's path from the top of its repository, with a
/// trailing `/`, such as `tracker/`, or empty when the root is the top.
pub fn prefix(root: &Path) -> Result<String> {
    Ok(git(root, &["rev-parse", "--show-prefix"])?
        .trim_end()
        .to_string())
}

/// One file one commit touched, for the triggers (§2.1, §2.5).
#[derive(Debug, Clone, PartialEq)]
pub struct FileChange {
    /// The commit that touched it, as a full SHA.
    pub sha: String,
    /// The file's path from the top of the repository.
    pub path: String,
    /// Git's status letter: `A` added, `D` deleted, `M` modified, `T` its
    /// type changed. Renames show as a delete and an add.
    pub status: char,
    /// Lines added and removed, or `None` for a binary file.
    pub lines: Option<(u64, u64)>,
}

/// Every file each commit in `shas` touched, inside the project's root or
/// not, in the order given, as git names them from the top of the
/// repository. A merge is compared with its first parent, and a root
/// commit with nothing.
pub fn changes(root: &Path, shas: &[String]) -> Result<Vec<FileChange>> {
    if shas.is_empty() {
        return Ok(Vec::new());
    }
    let mut args = vec![
        "-c",
        "log.showRoot=true",
        "show",
        "-z",
        "--no-renames",
        "--diff-merges=first-parent",
        "--no-ext-diff",
        "--no-textconv",
        "--no-relative",
        "--format=%x01%H",
        "--raw",
        "--numstat",
    ];
    args.extend(shas.iter().map(|sha| sha.as_str()));
    let text = git(root, &args)?;
    // With `-z`, each commit starts with `\x01<sha>`, then each file's raw
    // line, such as `:100644 100644 <old> <new> M`, is followed by its
    // path, and then each file's numstat entry is `<added>\t<removed>\t<path>`,
    // where `-` counts mean a binary file.
    let mut all = Vec::new();
    let mut commit: Vec<FileChange> = Vec::new();
    let mut sha = String::new();
    let mut tokens = text.split('\0');
    while let Some(token) = tokens.next() {
        let token = token.trim_start_matches('\n');
        if token.starts_with('\x01') {
            all.append(&mut commit);
            sha = token.trim_start_matches('\x01').to_string();
        } else if token.starts_with(':') {
            let status = token
                .split_whitespace()
                .last()
                .and_then(|field| field.chars().next())
                .unwrap_or('M');
            let path = tokens.next().unwrap_or_default().to_string();
            commit.push(FileChange {
                sha: sha.clone(),
                path,
                status,
                lines: None,
            });
        } else if let [added, removed, path] = token.splitn(3, '\t').collect::<Vec<_>>()[..] {
            let lines = added.parse().ok().zip(removed.parse().ok());
            if let Some(change) = commit.iter_mut().find(|change| change.path == path) {
                change.lines = lines;
            }
        }
    }
    all.append(&mut commit);
    Ok(all)
}

/// The commits `HEAD` reaches that `base` doesn't, as full SHAs.
pub fn since(root: &Path, base: &str) -> Result<Vec<String>> {
    Ok(git(root, &["rev-list", &since_range(base)])?
        .lines()
        .map(str::to_string)
        .collect())
}

/// The range of commits made since `base`, as `since` and `unscoped` read
/// it, so T4 and T14 agree on what "after start" means: a commit `base`
/// can't reach, which includes every commit rewritten since then (§2.5).
fn since_range(base: &str) -> String {
    format!("{base}..HEAD")
}

/// Whether `header`, a commit message's first line, is scoped to some
/// item: `<type>(<ID>): …`, where the ID is a project key, a dash and a
/// number, as `is_item_header` matches it (§2.1). A commit that isn't is
/// unscoped (T14).
pub fn is_scoped_header(header: &str) -> bool {
    let Some(scope) = header
        .split_once('(')
        .and_then(|(_, rest)| rest.split_once(')'))
        .map(|(scope, _)| scope)
    else {
        return false;
    };
    let Some((key, number)) = scope.split_once('-') else {
        return false;
    };
    crate::config::is_valid_key(key)
        && !number.is_empty()
        && number.bytes().all(|b| b.is_ascii_digit())
        && is_item_header(header, scope)
}

/// Whether a commit headed `header` is another project's work: scoped to
/// an item whose key is one of `projects`, other than item `id`'s own
/// project. A scope in a key that isn't a registered project isn't, so it
/// counts as unscoped.
fn is_another_projects(header: &str, id: &str, projects: &[String]) -> bool {
    let scope = header
        .split_once('(')
        .and_then(|(_, rest)| rest.split_once(')'))
        .map_or("", |(scope, _)| scope);
    let key = |text: &str| text.split_once('-').map_or("", |(key, _)| key).to_string();
    key(scope) != key(id) && projects.contains(&key(scope))
}

/// The unscoped commits made since `base` that touch `root` (T14), each as
/// its full SHA and header, newest first. With no `base`, as for an item
/// started in a repository with no commits yet, every commit `HEAD`
/// reaches counts. A commit that only touches files outside the root
/// doesn't, and neither does a commit scoped to item `id` or to the key of
/// another project in `projects`. One scoped to another item of `id`'s
/// project counts, since it gets past every trigger that reads this item's
/// changed files, and so does one scoped to a key that isn't in `projects`.
pub fn unscoped(
    root: &Path,
    id: &str,
    base: Option<&str>,
    projects: &[String],
) -> Result<Vec<(String, String)>> {
    if head(root)?.is_none() {
        return Ok(Vec::new());
    }
    let range = base.map_or("HEAD".to_string(), since_range);
    // One record per commit, ended by a NUL, as `item_log` reads them. The
    // `.` limits it to commits that touch the root, the current folder,
    // and `--full-history` keeps each one, even where a merge would hide
    // it.
    let text = git(
        root,
        &[
            "log",
            "-z",
            "--full-history",
            "--format=%H%n%B",
            &range,
            "--",
            ".",
        ],
    )?;
    let commits = text
        .split('\0')
        .filter_map(|record| {
            let mut lines = record.trim_start_matches('\n').lines();
            let sha = lines.next()?;
            let header = lines.next().unwrap_or_default();
            let own = is_item_header(header, id);
            let other = is_scoped_header(header) && is_another_projects(header, id, projects);
            (!own && !other).then(|| (sha.to_string(), header.to_string()))
        })
        .collect();
    Ok(commits)
}

/// Whether `path`, from the top of the repository, is a file at commit
/// `sha`.
pub fn exists_at(root: &Path, sha: &str, path: &str) -> bool {
    git(root, &["cat-file", "-e", &format!("{sha}:{path}")]).is_ok()
}

/// The ref that keeps item `id`'s verified commit (§2.7).
pub fn ref_name(id: &str) -> String {
    format!("refs/tracker/{id}")
}

/// Points `refs/tracker/<id>` at `sha`, creating it or moving it.
pub fn set_ref(root: &Path, id: &str, sha: &str) -> Result<()> {
    git(root, &["update-ref", &ref_name(id), sha]).map(|_| ())
}

/// Whether `refs/tracker/<id>` exists in the repository that holds `root`.
pub fn ref_exists(root: &Path, id: &str) -> Result<bool> {
    let output = Command::new("git")
        .current_dir(root)
        .args(["show-ref", "--verify", "--quiet", &ref_name(id)])
        .stdin(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map_err(|e| Error::store(format!("Can't run git in {}: {e}.", root.display())))?;
    Ok(output.success())
}

/// The commit `refs/tracker/<id>` points to, as a full SHA, or `None` if
/// the ref doesn't exist. `audit` compares it with the record's `commit`.
pub fn ref_target(root: &Path, id: &str) -> Result<Option<String>> {
    if !ref_exists(root, id)? {
        return Ok(None);
    }
    let target = format!("{}^{{commit}}", ref_name(id));
    let sha = git(root, &["rev-parse", "--verify", "--quiet", &target])?;
    Ok(Some(sha.trim().to_string()))
}

/// Removes `refs/tracker/<id>`.
pub fn delete_ref(root: &Path, id: &str) -> Result<()> {
    git(root, &["update-ref", "-d", &ref_name(id)]).map(|_| ())
}

#[cfg(test)]
mod tests {
    use super::{is_item_header, is_scoped_header};

    #[test]
    fn a_header_is_scoped_when_its_scope_is_any_items_id() {
        for header in [
            "feat(BM-1): add search",
            "fix(TR2-40)!: drop the old sort flag",
            "docs(XX-12): say so",
        ] {
            assert!(is_scoped_header(header), "{header}");
        }
        for header in [
            "BM-1: add search",
            "tidy the notes",
            "chore: tidy",
            "feat(search): add search",
            "feat(bm-1): add search",
            "feat(BM-): add search",
            "feat(BM-1a): add search",
            "feat(BM-1):add search",
            "Merge branch 'feat(BM-1): x'",
            "",
        ] {
            assert!(!is_scoped_header(header), "{header}");
        }
    }

    #[test]
    fn a_header_is_scoped_to_an_item_only_by_its_conventional_commit_scope() {
        for header in [
            "feat(BM-1): add search",
            "fix(BM-1)!: drop the old sort flag",
            "test(BM-1): add the tests",
        ] {
            assert!(is_item_header(header, "BM-1"), "{header}");
        }
        for header in [
            "feat(BM-12): sort by date",
            "BM-1: add search",
            "feat(BM-1):add search",
            "feat(BM-1) add search",
            "Feat(BM-1): add search",
            "(BM-1): add search",
            "feat(bm-1): add search",
            "feat(BM-1)!!: add search",
            " feat(BM-1): add search",
            "chore: tidy",
            "",
        ] {
            assert!(!is_item_header(header, "BM-1"), "{header}");
        }
    }
}
