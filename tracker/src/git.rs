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
    let inside = Command::new("git")
        .current_dir(root)
        .args(["rev-parse", "--is-inside-work-tree"])
        .stdin(Stdio::null())
        .stderr(Stdio::null())
        .output();
    match inside {
        Ok(output) if output.status.success() && output.stdout.starts_with(b"true") => Ok(()),
        Ok(_) => Err(Error::store(format!(
            "{key}'s root, {}, isn't inside a git work tree, so tracker can't record or prove \
             {id}'s work against a commit. Make it a repository with `git init`, or register \
             {key} with a root that's inside one. Nothing was changed.",
            root.display()
        ))),
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
/// wrong with it, such as `modified xx/a.txt`. Files that `.gitignore`
/// excludes, and files outside the root, aren't listed (G5, §2.1).
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
            is_item_header(header, id).then(|| sha.to_string())
        })
        .collect();
    Ok(commits)
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

/// Removes `refs/tracker/<id>`.
pub fn delete_ref(root: &Path, id: &str) -> Result<()> {
    git(root, &["update-ref", "-d", &ref_name(id)]).map(|_| ())
}

#[cfg(test)]
mod tests {
    use super::is_item_header;

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
