//! Where the store lives, its lock, and reading and writing its files
//! (§4.1, §4.4).

use std::collections::BTreeMap;
use std::fs::{self, File, OpenOptions};
use std::io::{ErrorKind, Write};
use std::path::{Path, PathBuf};

use crate::error::{self, Error, Result};
use crate::item::{self, Item};

pub struct Store {
    pub home: PathBuf,
}

/// What `init` did.
pub struct Init {
    /// True when there was no store here before.
    pub created: bool,
    /// The parts of an existing store that were missing and have been put
    /// back, such as `counters.json`. Empty for a new store.
    pub repaired: Vec<&'static str>,
}

impl Store {
    /// The store's folder: `TRACKER_HOME` if it's set, otherwise `~/.tracker`.
    /// A relative `TRACKER_HOME` is refused, because it would name a
    /// different store in every folder a command runs from.
    pub fn locate() -> Result<Store> {
        if let Some(home) = std::env::var_os("TRACKER_HOME")
            && !home.is_empty()
        {
            let home = PathBuf::from(home);
            if home.is_relative() {
                return Err(Error::input(format!(
                    "TRACKER_HOME is {}, a relative path, which would mean a different store \
                     in every folder. Set it to an absolute path, such as {}.",
                    home.display(),
                    std::env::current_dir()
                        .map(|cwd| cwd.join(&home).display().to_string())
                        .unwrap_or_else(|_| "/Users/me/.tracker".to_string())
                )));
            }
            check_not_a_file(
                &home,
                "TRACKER_HOME names",
                "Point TRACKER_HOME at a folder",
            )?;
            return Ok(Store { home });
        }
        match std::env::var_os("HOME") {
            Some(home) => {
                let home = PathBuf::from(home).join(".tracker");
                check_not_a_file(
                    &home,
                    "TRACKER_HOME isn't set, so the store is",
                    "Set TRACKER_HOME to a folder",
                )?;
                Ok(Store { home })
            }
            None => Err(Error::store(
                "Neither TRACKER_HOME nor HOME is set, so there's no store to use. Set \
                 TRACKER_HOME to the store's folder, an absolute path such as /Users/me/.tracker."
                    .to_string(),
            )),
        }
    }

    pub fn config_path(&self) -> PathBuf {
        self.home.join("config.json")
    }

    pub fn counters_path(&self) -> PathBuf {
        self.home.join("counters.json")
    }

    pub fn lock_path(&self) -> PathBuf {
        self.home.join("lock")
    }

    pub fn items_dir(&self) -> PathBuf {
        self.home.join("items")
    }

    /// One line per deleted item (§2.8), created by the first `delete`.
    pub fn deleted_path(&self) -> PathBuf {
        self.home.join("deleted.json")
    }

    /// The item file for `id`. Anything that isn't an item ID, such as
    /// `../config`, is refused, so no ID can name a file outside `items/`.
    pub fn item_path(&self, id: &str) -> Result<PathBuf> {
        if !item::is_valid_id(id) {
            return Err(Error::input(format!(
                "\"{id}\" isn't an item ID. An ID is a project key, a dash and a number, \
                 such as BM-12."
            )));
        }
        Ok(self.items_dir().join(format!("{id}.json")))
    }

    /// Creates whatever parts of the store are missing and leaves the rest
    /// alone, and says which parts it put back. The files that are already
    /// there are checked first, so a store that can't be used isn't
    /// reported as fine, and nothing is written to it.
    pub fn init(&self) -> Result<Init> {
        create_private_dir(&self.home).map_err(|e| error::io_in_store("create", &self.home, &e))?;
        let _lock = self.lock()?;

        let config_missing = is_missing(&self.config_path())?;
        let counters_missing = is_missing(&self.counters_path())?;
        let items_missing = is_missing(&self.items_dir())?;
        if !config_missing {
            crate::config::load(self)?;
        }
        if !counters_missing {
            self.read_counters()?;
        }

        let mut put_back = Vec::new();
        if config_missing {
            write_file(&self.config_path(), "{\n  \"projects\": {}\n}\n")?;
            put_back.push("config.json");
        }
        if counters_missing {
            write_file(&self.counters_path(), "{}\n")?;
            put_back.push("counters.json");
        }
        if items_missing {
            let items = self.items_dir();
            fs::create_dir(&items).map_err(|e| error::io_in_store("create", &items, &e))?;
            put_back.push("items/");
        }

        let created = config_missing && counters_missing && items_missing;
        Ok(Init {
            created,
            repaired: if created { Vec::new() } else { put_back },
        })
    }

    /// Fails with "Run `tracker init`." (exit 1) until the store has been
    /// created, or when part of it is missing, which `init` puts back. Fails
    /// with exit 3 if the store is there but can't be read, or if
    /// `config.json` can't be parsed, even for a command that doesn't use
    /// it (§3): a broken store is repaired by hand first, and a command that
    /// carried on would hide that it's broken.
    pub fn require_exists(&self) -> Result<()> {
        // `init` creates these three, and puts back whichever is missing.
        let parts = [
            (self.config_path(), false),
            (self.counters_path(), false),
            (self.items_dir(), true),
        ];
        for (i, (path, is_dir)) in parts.iter().enumerate() {
            match fs::metadata(path) {
                Ok(meta) if meta.is_dir() == *is_dir => {}
                Ok(_) => {
                    let wanted = if *is_dir { "a folder" } else { "a file" };
                    return Err(Error::store(format!(
                        "{} should be {wanted}, so the store can't be read.",
                        path.display()
                    )));
                }
                // No config.json, and none of the rest, means no store at
                // all. A store with only config.json missing gets the
                // message below, since init puts it back.
                Err(e)
                    if e.kind() == ErrorKind::NotFound
                        && i == 0
                        && !self.counters_path().exists()
                        && !self.items_dir().exists() =>
                {
                    return Err(Error::input(format!(
                        "There's no tracker store at {}. Run `tracker init`.",
                        self.home.display()
                    )));
                }
                Err(e) if e.kind() == ErrorKind::NotFound => {
                    return Err(Error::input(format!(
                        "The store at {} is missing {}. Run `tracker init` to put it back.",
                        self.home.display(),
                        path.display()
                    )));
                }
                Err(e) if e.kind() == ErrorKind::PermissionDenied => {
                    let home = self.home.display();
                    let quoted = error::shell_quote(&self.home);
                    return Err(Error::store(format!(
                        "Can't read the store at {home}: {e}. Check its permissions with \
                         `ls -ld {quoted}`. If it's yours, `chmod u+rwx {quoted}` lets you read \
                         and write it again."
                    )));
                }
                Err(e) => return Err(error::io("read", path, &e)),
            }
        }
        crate::config::load(self)?;
        Ok(())
    }

    /// Takes the exclusive lock on the store's `lock` file, waiting while
    /// another tracker command holds it. The lock is released when the
    /// returned file is dropped, or by the operating system if the command
    /// dies, so it can never go stale.
    pub fn lock(&self) -> Result<File> {
        let path = self.lock_path();
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(&path)
            .map_err(|e| error::io_in_store("open", &path, &e))?;
        file.lock()
            .map_err(|e| error::io_in_store("lock", &path, &e))?;
        Ok(file)
    }

    /// Takes the lock, then checks that item `id`'s file still holds exactly
    /// `before`, the text it held when the command read it without the
    /// lock. Returns the lock, which is held until it's dropped.
    ///
    /// A human-only command asks its question without the lock (§2.7),
    /// since holding it would stop every other command while I decide, and
    /// `approve` (slice 9) runs the gates in between too. So each one calls
    /// this before it writes, and whatever the warning described is what
    /// it changes. Refuses with exit 1, writing nothing, if the item was
    /// removed or changed meanwhile. `command` is how it was run, such as
    /// `tracker delete XX-1`, so the message can say what to run again.
    pub fn lock_unchanged(&self, id: &str, before: &str, command: &str) -> Result<File> {
        let lock = self.lock()?;
        let now = match self.item_text(id) {
            Ok(text) => text,
            Err(err) if err.code == 1 => {
                return Err(Error::input(format!(
                    "{id} was removed while you were deciding. Nothing was changed."
                )));
            }
            Err(err) => return Err(err),
        };
        if now != before {
            return Err(Error::input(format!(
                "{id} changed while you were deciding, so nothing was changed. Run `{command}` \
                 again to see what it is now."
            )));
        }
        Ok(lock)
    }

    /// Takes the lock again after checks ran without it (§4.4), and checks
    /// that item `id`'s file still holds exactly `before`, the text it held
    /// when the lock was let go. A check can run for minutes, so `start`,
    /// `submit` and `gates` never hold the lock while one runs. Refuses with
    /// exit 1, writing nothing, if the item was removed or changed
    /// meanwhile. `command` is how it was run, such as `tracker submit
    /// XX-1`, so the message can say what to run again.
    pub fn lock_unchanged_after_checks(
        &self,
        id: &str,
        before: &str,
        command: &str,
    ) -> Result<File> {
        let lock = self.lock()?;
        let now = match self.item_text(id) {
            Ok(text) => text,
            Err(err) if err.code == 1 => {
                return Err(Error::input(format!(
                    "{id} was removed while its checks ran. Nothing was changed."
                )));
            }
            Err(err) => return Err(err),
        };
        if now != before {
            return Err(Error::input(format!(
                "{id} changed while its checks ran, so nothing was changed. Run `{command}` again."
            )));
        }
        Ok(lock)
    }

    /// Hands out the next ID for `project` from `counters.json` and records
    /// the number after it. IDs never come from the item files, so a deleted
    /// item's ID is never reused. Call this only while holding the lock.
    ///
    /// The item files and `deleted.json` are a tripwire, though: a counter
    /// at or below a number already used means `counters.json` was lost or
    /// reset, and handing out its next number could reuse a deleted item's
    /// ID, whose scoped commits would then count as the new item's work
    /// (§2.8).
    pub fn next_id(&self, project: &str) -> Result<String> {
        let path = self.counters_path();
        let mut counters = self.read_counters()?;

        let number = counters.get(project).copied().unwrap_or(1);
        if let Some((highest, place)) = self.highest_number_used(project)?
            && number <= highest
        {
            return Err(Error::store(format!(
                "{} says the next {project} item is {project}-{number}, but {project}-{highest} \
                 {place}, so the counter was probably lost or reset. Set \"{project}\" in \
                 counters.json above the highest {project} number ever used: at least {}.",
                path.display(),
                highest.saturating_add(1)
            )));
        }

        let Some(after) = number.checked_add(1) else {
            return Err(Error::store(format!(
                "{} gives {project} the counter {number}, the largest number tracker can count \
                 to, so there's no number left after it. Set it to the highest {project} \
                 number ever used, plus 1.",
                path.display()
            )));
        };
        counters.insert(project.to_string(), after);
        let text = serde_json::to_string_pretty(&counters).expect("counters always serialize");
        write_file(&path, &format!("{text}\n"))?;
        Ok(format!("{project}-{number}"))
    }

    /// Reads `counters.json`: the next number for each project. A counter
    /// below 1 is refused, since IDs start at 1 and `XX-0` isn't an ID.
    fn read_counters(&self) -> Result<BTreeMap<String, u64>> {
        let path = self.counters_path();
        let text = read_file(&path)?;
        let counters: BTreeMap<String, u64> = serde_json::from_str(&text).map_err(|e| {
            error::parse(
                &path,
                &e,
                "It maps each project key to its next item number, such as {\"BM\": 13}, so \
                 fix it by hand. Set each counter above the highest number that project has \
                 used, in items/ or deleted.json, so no ID is handed out twice.",
            )
        })?;
        if let Some((key, _)) = counters.iter().find(|(_, next)| **next == 0) {
            return Err(Error::store(format!(
                "{} gives {key} the counter 0, but IDs start at 1. Set it above the highest \
                 {key} number ever used, or to 1 if there's never been one.",
                path.display()
            )));
        }
        Ok(counters)
    }

    /// The highest number `project` has used, among its item files and the
    /// IDs in `deleted.json`, with where it was found, such as
    /// `(7, "already exists")` when `BM-7.json` is the highest. `None` if
    /// it has used none.
    fn highest_number_used(&self, project: &str) -> Result<Option<(u64, &'static str)>> {
        let prefix = format!("{project}-");
        let number_of = |id: &str| id.strip_prefix(&prefix).and_then(|n| n.parse::<u64>().ok());

        let dir = self.items_dir();
        let entries = fs::read_dir(&dir).map_err(|e| error::io_in_store("read", &dir, &e))?;
        let mut on_disk = None;
        for entry in entries {
            let entry = entry.map_err(|e| error::io("read", &dir, &e))?;
            let name = entry.file_name();
            let number = name
                .to_str()
                .and_then(|name| name.strip_suffix(".json"))
                .and_then(number_of);
            on_disk = on_disk.max(number);
        }

        // A line that isn't a delete record is skipped: this is a tripwire,
        // and `create` shouldn't stop over a bad line it doesn't need.
        let mut deleted = None;
        for line in self.read_deleted()?.lines() {
            let id = serde_json::from_str::<serde_json::Value>(line)
                .ok()
                .and_then(|record| record["id"].as_str().and_then(number_of));
            deleted = deleted.max(id);
        }

        // `None` counts as less than any number.
        Ok(if deleted > on_disk {
            deleted.map(|number| (number, "was used by an item since deleted"))
        } else {
            on_disk.map(|number| (number, "already exists"))
        })
    }

    /// The text of `deleted.json`, or "" before the first delete.
    fn read_deleted(&self) -> Result<String> {
        let path = self.deleted_path();
        match fs::read_to_string(&path) {
            Ok(text) => Ok(text),
            Err(e) if e.kind() == ErrorKind::NotFound => Ok(String::new()),
            Err(e) => Err(error::io("read", &path, &e)),
        }
    }

    /// The text of an item's file, exactly as it is on disk. Fails with
    /// exit 1 if there's no such item, and exit 3 if its file isn't a
    /// regular file once symlinks are followed, such as a symlink that
    /// leads nowhere or a pipe, which is never read (§4.4).
    pub fn item_text(&self, id: &str) -> Result<String> {
        let path = self.item_path(id)?;
        let no_item = || {
            Error::input(format!(
                "There's no item {id}. `tracker list --all --status \
                 open,in-progress,review,done,shelved` lists every item."
            ))
        };
        match entry_at(&path)? {
            Entry::File => {}
            Entry::Gone => return Err(no_item()),
            Entry::Other(problem) => return Err(problem.error()),
        }
        match fs::read_to_string(&path) {
            Ok(text) => Ok(text),
            // Removed since `entry_at` looked.
            Err(e) if e.kind() == ErrorKind::NotFound => Err(no_item()),
            Err(e) if e.kind() == ErrorKind::InvalidData => Err(not_utf8(&path).error()),
            Err(e) => Err(error::io("read", &path, &e)),
        }
    }

    /// Deletes item `id` for good (§2.8): adds `record`, one line of JSON,
    /// to `deleted.json`, then removes the item's file. Call this only while
    /// holding the lock.
    ///
    /// The record is written first, so a crash between the two leaves a
    /// delete that's listed but didn't happen, which I'd notice, rather than
    /// one that happened unlisted. If the file can't be removed, the record
    /// is taken back out.
    ///
    /// Running the delete again after such a crash finds the item's line
    /// already there, so it only removes the file, and the digest doesn't
    /// list one delete twice. IDs are never reused, so a line with this ID
    /// can only be from that delete.
    pub fn delete_item(&self, id: &str, record: &str) -> Result<()> {
        let path = self.item_path(id)?;
        let log = self.deleted_path();
        let before = self.read_deleted()?;
        let already_listed = before.lines().any(|line| {
            serde_json::from_str::<serde_json::Value>(line)
                .is_ok_and(|earlier| earlier["id"].as_str() == Some(id))
        });
        if !already_listed {
            let mut text = before.clone();
            if !text.is_empty() && !text.ends_with('\n') {
                text.push('\n');
            }
            text.push_str(record);
            text.push('\n');
            write_file(&log, &text)?;
        }

        if let Err(e) = fs::remove_file(&path) {
            if already_listed {
                return Err(Error::store(format!(
                    "Can't remove {}: {e}. deleted.json already lists it, from an earlier \
                     delete that didn't finish.",
                    path.display()
                )));
            }
            let put_back = if before.is_empty() {
                fs::remove_file(&log).is_ok()
            } else {
                write_file(&log, &before).is_ok()
            };
            let record_note = if put_back {
                "Nothing was deleted."
            } else {
                "deleted.json lists it anyway, so remove its last line by hand."
            };
            return Err(Error::store(format!(
                "Can't remove {}: {e}. {record_note}",
                path.display()
            )));
        }
        // Flushing the folder keeps the removal, as in `write_file`.
        if let Ok(dir) = File::open(self.items_dir()) {
            let _ = dir.sync_all();
        }
        Ok(())
    }

    /// Writes a new item's file, refusing one that already exists. Call
    /// this only while holding the lock.
    pub fn save_item(&self, item: &Item) -> Result<()> {
        let path = self.item_path(&item.id)?;
        if fs::symlink_metadata(&path).is_ok() {
            return Err(Error::store(format!(
                "{} already exists, so {} can't be created. Nothing was written.",
                path.display(),
                item.id
            )));
        }
        write_file(&path, &item::to_json(item))
    }

    /// Replaces an existing item's file with `item`. Call this only while
    /// holding the lock, after reading the item under it.
    ///
    /// Refuses, with exit 3, when the file on disk holds something this
    /// version doesn't know, such as a field a newer tracker added. Writing
    /// it back would drop that field without a word. A value that's only
    /// written differently, such as `"labels": []` or a time ending
    /// `+00:00` from a hand edit, is rewritten the way tracker writes it,
    /// since nothing is lost (`item::Lost::respelled`).
    pub fn update_item(&self, item: &Item) -> Result<()> {
        let path = self.item_path(&item.id)?;
        let text = self.item_text(&item.id)?;
        let lost = item::lost_on_rewrite(&text).map_err(|e| error::parse(&path, &e, ITEM_FIX))?;
        if !lost.unknown.is_empty() {
            return Err(Error::store(format!(
                "{} has fields this version of tracker doesn't know, so rewriting it would lose \
                 them: {}. It was probably written by a newer tracker, so use that one. Nothing \
                 was written.",
                path.display(),
                lost.unknown.join(", ")
            )));
        }
        write_file(&path, &item::to_json(item))
    }

    /// Reads one item by its ID, with its file's text exactly as it is on
    /// disk. Both come from one read, so the text is what the item was
    /// parsed from. Fails with exit 1 if there's no such item.
    pub fn load_item_with_text(&self, id: &str) -> Result<(Item, String)> {
        let text = self.item_text(id)?;
        let item = parse_item(&self.item_path(id)?, &text)?;
        Ok((item, text))
    }

    /// Reads one item by its ID. Fails with exit 1 if there's no such item.
    pub fn load_item(&self, id: &str) -> Result<Item> {
        Ok(self.load_item_with_text(id)?.0)
    }

    /// Reads every item file, sorted by project and then by number, and
    /// the files it skipped (§4.4). A file that can't be read as the item
    /// its name says, such as one that isn't valid JSON, costs that one
    /// item, not the whole list, so it's skipped and reported instead.
    /// Anything that isn't a regular file once symlinks are followed, such
    /// as a folder, a pipe or a symlink that leads nowhere, is skipped and
    /// reported the same way, without being read: a pipe would never
    /// finish. A file removed between listing the folder and reading it,
    /// by a `delete` that ran meanwhile, is left out without a word, since
    /// that item is gone. A folder or file the store can't read at all, such
    /// as one without permission, still fails, with exit 3: that's the
    /// store, not one item.
    pub fn load_items(&self) -> Result<(Vec<Item>, Vec<Skipped>)> {
        let dir = self.items_dir();
        let entries = fs::read_dir(&dir).map_err(|e| error::io_in_store("read", &dir, &e))?;

        let mut items = Vec::new();
        let mut skipped = Vec::new();
        for entry in entries {
            let path = entry.map_err(|e| error::io("read", &dir, &e))?.path();
            // Skips anything that isn't an item file, such as `XX-1.json.4242.tmp`.
            if path.extension().and_then(|ext| ext.to_str()) != Some("json") {
                continue;
            }
            match read_listed(&path)? {
                Listed::Item(item) => items.push(*item),
                Listed::Skipped(problem) => skipped.push(problem),
                Listed::Gone => {}
            }
        }
        items.sort_by(|a, b| (&a.project, a.number()).cmp(&(&b.project, b.number())));
        skipped.sort_by(|a, b| a.file.cmp(&b.file));
        Ok((items, skipped))
    }
}

/// What `load_items` made of one `.json` file in `items/`.
enum Listed {
    /// Boxed, since an item is far larger than the other variants.
    Item(Box<Item>),
    /// A file that can't be read as the item its name says.
    Skipped(Skipped),
    /// A file that was removed after the folder was listed.
    Gone,
}

/// Reads one file that `load_items` found in `items/`. Fails, with exit 3,
/// only when the file is there but can't be read at all, such as one
/// without permission.
fn read_listed(path: &Path) -> Result<Listed> {
    match entry_at(path)? {
        Entry::File => {}
        Entry::Gone => return Ok(Listed::Gone),
        Entry::Other(problem) => return Ok(Listed::Skipped(problem)),
    }
    let text = match fs::read_to_string(path) {
        Ok(text) => text,
        Err(e) if e.kind() == ErrorKind::InvalidData => return Ok(Listed::Skipped(not_utf8(path))),
        // Removed since `entry_at` looked.
        Err(e) if e.kind() == ErrorKind::NotFound => return Ok(Listed::Gone),
        Err(e) => return Err(error::io("read", path, &e)),
    };
    Ok(match check_item(path, &text) {
        Ok(item) => Listed::Item(Box::new(item)),
        Err(problem) => Listed::Skipped(problem),
    })
}

/// What to do about an item file that isn't valid JSON, or isn't an item.
const ITEM_FIX: &str = "It's JSON, laid out as in DESIGN.md §4.3, so fix it by hand, or move \
                        it out of items/ to set it aside.";

/// An item file that can't be read as the item its name says (§4.4):
/// where, and why. `load_items` skips it and reports it, and a command
/// that names its item, such as `show`, refuses with it.
#[derive(Debug, Clone, PartialEq)]
pub struct Skipped {
    pub file: PathBuf,
    /// Where the JSON parser stopped, if it did.
    pub line: Option<usize>,
    pub column: Option<usize>,
    /// Why, without the line and column, such as ``expected `,` or `}` ``.
    pub reason: String,
    /// What to do about it.
    pub fix: String,
}

impl Skipped {
    /// The file, where in it and why, such as
    /// ``…/XX-2.json, line 3, column 5: expected `,` or `}`.``
    pub fn describe(&self) -> String {
        format!(
            "{}: {}.",
            error::at(&self.file, self.line, self.column),
            self.reason
        )
    }

    /// The refusal for a command that names this item: exit 3.
    pub fn error(&self) -> Error {
        Error::store(format!("Can't read {} {}", self.describe(), self.fix))
    }
}

/// A file that isn't UTF-8, so it can't even be parsed.
fn not_utf8(path: &Path) -> Skipped {
    Skipped {
        file: path.to_path_buf(),
        line: None,
        column: None,
        reason: "it isn't UTF-8 text".to_string(),
        fix: ITEM_FIX.to_string(),
    }
}

/// What is at a path in `items/`, looked at before it's read.
enum Entry {
    /// A regular file, or a symlink to one: it can be read.
    File,
    /// Nothing: there's no such file, or it was removed.
    Gone,
    /// Something that can't be read as an item without being opened,
    /// such as a folder, a pipe or a symlink that leads nowhere.
    Other(Skipped),
}

/// Looks at `path`, following symlinks, without opening it. Only a regular
/// file is read: a pipe would make the read wait forever, while holding the
/// lock in a command that writes, and a device such as `/dev/zero` would
/// never end. Fails, with exit 3, when the store can't be read there, such
/// as a folder without permission.
fn entry_at(path: &Path) -> Result<Entry> {
    let skipped = |reason: String, fix: &str| {
        Entry::Other(Skipped {
            file: path.to_path_buf(),
            line: None,
            column: None,
            reason,
            fix: fix.to_string(),
        })
    };
    let followed = match fs::metadata(path) {
        Ok(meta) if meta.is_file() => return Ok(Entry::File),
        Ok(meta) if meta.is_dir() => {
            return Ok(skipped(
                "it's a folder, not a file".to_string(),
                "Move it out of items/.",
            ));
        }
        Ok(_) => {
            return Ok(skipped(
                "it isn't a regular file".to_string(),
                "Move it out of items/.",
            ));
        }
        Err(e) => e,
    };
    // It can't be followed. Either it isn't there at all, or it's a
    // symlink whose target is missing, loops or can't be reached.
    match fs::symlink_metadata(path) {
        Ok(link) if link.file_type().is_symlink() => {
            // A target in a folder without permission is the store's
            // problem, like any file without permission.
            if followed.kind() == ErrorKind::PermissionDenied {
                return Err(error::io("read", path, &followed));
            }
            let reason = if followed.kind() == ErrorKind::NotFound {
                "it's a symlink to a file that doesn't exist".to_string()
            } else {
                format!("it's a symlink that can't be followed: {followed}")
            };
            Ok(skipped(reason, "Remove it from items/."))
        }
        Err(e) if e.kind() == ErrorKind::NotFound => Ok(Entry::Gone),
        _ => Err(error::io("read", path, &followed)),
    }
}

/// Parses `text`, read from `path`, and checks that it is the item its
/// name says: the file `XX-1.json` must hold `"id": "XX-1"` in project
/// `XX`. A file copied or renamed by hand would otherwise list one ID
/// twice, and a later write to `item_path(&item.id)` would overwrite the
/// other item's file. Fails with exit 3, naming the file, the line and the
/// reason.
fn parse_item(path: &Path, text: &str) -> Result<Item> {
    check_item(path, text).map_err(|problem| problem.error())
}

/// `parse_item`'s checks, returning what's wrong as a `Skipped`.
fn check_item(path: &Path, text: &str) -> std::result::Result<Item, Skipped> {
    let problem = |line, column, reason: String, fix: &str| Skipped {
        file: path.to_path_buf(),
        line,
        column,
        reason,
        fix: fix.to_string(),
    };
    let item = item::from_json(text).map_err(|e| {
        let (line, column, reason) = error::json_problem(&e);
        problem(line, column, reason, ITEM_FIX)
    })?;
    let named = path
        .file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or_default();
    if !item::is_valid_id(named) {
        return Err(problem(
            None,
            None,
            "it isn't named after an item ID, such as BM-12.json".to_string(),
            "Move it out of items/.",
        ));
    }
    if item.id != named {
        return Err(problem(
            None,
            None,
            format!(
                "it holds the item \"{}\", but its name says {named}",
                item.id
            ),
            "Rename the file or fix its \"id\", and check that no other file holds the same ID.",
        ));
    }
    let key = named
        .split_once('-')
        .map(|(key, _)| key)
        .unwrap_or_default();
    if item.project != key {
        return Err(problem(
            None,
            None,
            format!(
                "it's in project \"{}\", but its ID {named} says {key}",
                item.project
            ),
            "Fix its \"project\".",
        ));
    }
    Ok(item)
}

/// Refuses, with exit 3, a store folder `home` that's a file or a symlink
/// that can't be followed, or that has one where one of the folders above
/// it should be. Otherwise every command would fail with "Not a directory"
/// on a file inside it, or say "Run `tracker init`" for a symlink to
/// nowhere, and `init` would fail with "File exists", none of which says
/// what's wrong. `named` says where `home` came from, and `instead` what
/// else to do than move the file or create the link's target.
fn check_not_a_file(home: &Path, named: &str, instead: &str) -> Result<()> {
    for path in home.ancestors() {
        // What's at `path`, whether it's a file rather than a symlink, and
        // the fix.
        let (what, is_file, fix) = match fs::metadata(path) {
            // The folders above a folder are folders too.
            Ok(meta) if meta.is_dir() => return Ok(()),
            Ok(_) => (
                "a file, not a folder".to_string(),
                true,
                "move the file".to_string(),
            ),
            Err(e) => {
                let is_link =
                    fs::symlink_metadata(path).is_ok_and(|meta| meta.file_type().is_symlink());
                let Some(target) = fs::read_link(path).ok().filter(|_| is_link) else {
                    // Not there yet, which `init` puts right.
                    continue;
                };
                // A relative link is followed from the folder it's in.
                let target = match path.parent() {
                    Some(parent) if target.is_relative() => parent.join(target),
                    _ => target,
                };
                if e.kind() == ErrorKind::NotFound {
                    (
                        format!("a symlink to {}, which doesn't exist", target.display()),
                        false,
                        format!("create {}", target.display()),
                    )
                } else {
                    (
                        format!(
                            "a symlink to {}, which can't be followed: {e}",
                            target.display()
                        ),
                        false,
                        "fix the symlink".to_string(),
                    )
                }
            }
        };
        // "names X, which is a file", but "names X, a symlink to Y, which
        // doesn't exist", rather than two "which"es.
        let problem = match (path == home, is_file) {
            (true, true) => format!("which is {what}"),
            (true, false) => what,
            (false, _) => format!("but {} is {what}", path.display()),
        };
        return Err(Error::store(format!(
            "{named} {}, {problem}, so it can't hold the store. {instead}, or {fix}.",
            home.display()
        )));
    }
    Ok(())
}

/// Creates `path`, and any missing folders above it, readable only by me
/// (0700), since items hold my notes and, later, check output. A folder that
/// already exists is left as it is: `TRACKER_HOME` might name a folder that
/// has other uses.
fn create_private_dir(path: &Path) -> std::io::Result<()> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::DirBuilderExt;
        fs::DirBuilder::new()
            .recursive(true)
            .mode(0o700)
            .create(path)
    }
    #[cfg(not(unix))]
    {
        fs::create_dir_all(path)
    }
}

/// Whether `path` doesn't exist. Any error other than "not found", such as
/// a folder that can't be read, exits 3 rather than counting as missing.
fn is_missing(path: &Path) -> Result<bool> {
    match fs::symlink_metadata(path) {
        Ok(_) => Ok(false),
        Err(e) if e.kind() == ErrorKind::NotFound => Ok(true),
        Err(e) => Err(error::io("read", path, &e)),
    }
}

/// Reads `config.json` or `counters.json`, which `init` creates.
pub fn read_file(path: &Path) -> Result<String> {
    fs::read_to_string(path).map_err(|e| error::io_in_store("read", path, &e))
}

/// Writes `contents` to a temporary file beside `path`, then renames it over
/// `path`, so nobody ever reads a half-written file (§4.4).
///
/// The temporary file, such as `XX-1.json.4242.tmp`, is named with this
/// process's ID and must not exist yet. A symlink planted at that name is
/// refused rather than followed, so a write can't be sent to a file
/// outside the store.
pub fn write_file(path: &Path, contents: &str) -> Result<()> {
    let mut tmp_name = path.file_name().unwrap_or_default().to_os_string();
    tmp_name.push(format!(".{}.tmp", std::process::id()));
    let tmp = path.with_file_name(tmp_name);

    let mut file = match OpenOptions::new().write(true).create_new(true).open(&tmp) {
        Ok(file) => file,
        Err(e) if e.kind() == ErrorKind::AlreadyExists => {
            // Not ours, so it's left where it is.
            return Err(Error::store(format!(
                "Can't write {}: {} already exists. If no tracker is running, remove it and \
                 try again.",
                path.display(),
                tmp.display()
            )));
        }
        Err(e) => return Err(error::io_in_store("write", &tmp, &e)),
    };
    // Flushed to disk before the rename. Otherwise a crash could keep the
    // rename but lose the data, leaving an empty file in place of the old one.
    let written = file
        .write_all(contents.as_bytes())
        .and_then(|()| file.sync_all());
    if let Err(e) = written {
        let _ = fs::remove_file(&tmp);
        return Err(error::io_in_store("write", &tmp, &e));
    }
    if let Err(e) = fs::rename(&tmp, path) {
        let _ = fs::remove_file(&tmp);
        return Err(error::io_in_store("write", path, &e));
    }
    // Flushing the folder keeps the rename itself. This is best effort,
    // because not every system lets a folder be opened and flushed.
    if let Some(dir) = path.parent()
        && let Ok(dir) = File::open(dir)
    {
        let _ = dir.sync_all();
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    // `load_items` lists the folder and then reads each file, without the
    // lock, so a `delete` can remove a file in between. An integration test
    // can't time that, so this reads a file that was listed and is gone.
    #[test]
    fn a_file_removed_after_listing_is_left_out_without_a_word() {
        let dir = tempfile::TempDir::new().unwrap();
        let path = dir.path().join("XX-1.json");
        fs::write(&path, "{}").unwrap();
        fs::remove_file(&path).unwrap();
        assert!(matches!(read_listed(&path), Ok(Listed::Gone)));
    }
}
