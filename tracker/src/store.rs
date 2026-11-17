//! Where the store lives, its lock, and reading and writing its files
//! (§4.1, §4.4).

use std::collections::BTreeMap;
use std::fs::{self, File, OpenOptions};
use std::io::{ErrorKind, Write};
use std::path::{Path, PathBuf};

use crate::error::{Error, Result};
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
            return Ok(Store { home });
        }
        match std::env::var_os("HOME") {
            Some(home) => Ok(Store {
                home: PathBuf::from(home).join(".tracker"),
            }),
            None => Err(Error::store(
                "Neither TRACKER_HOME nor HOME is set, so there's no store to use.".to_string(),
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
        create_private_dir(&self.home)
            .map_err(|e| Error::store(format!("Can't create {}: {e}", self.home.display())))?;
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
            fs::create_dir(&items)
                .map_err(|e| Error::store(format!("Can't create {}: {e}", items.display())))?;
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
    /// with exit 3 if the store is there but can't be read.
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
                // No config.json means no store at all.
                Err(e) if e.kind() == ErrorKind::NotFound && i == 0 => {
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
                Err(e) => {
                    return Err(Error::store(format!(
                        "Can't read the store at {}: {e}. Check that you can read and write \
                         that folder.",
                        self.home.display()
                    )));
                }
            }
        }
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
            .map_err(|e| Error::store(format!("Can't open {}: {e}", path.display())))?;
        file.lock()
            .map_err(|e| Error::store(format!("Can't lock {}: {e}", path.display())))?;
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
        let counters: BTreeMap<String, u64> = serde_json::from_str(&text)
            .map_err(|e| Error::store(format!("Can't parse {}: {e}", path.display())))?;
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
        let entries = fs::read_dir(&dir)
            .map_err(|e| Error::store(format!("Can't read {}: {e}", dir.display())))?;
        let mut on_disk = None;
        for entry in entries {
            let entry =
                entry.map_err(|e| Error::store(format!("Can't read {}: {e}", dir.display())))?;
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
            Err(e) => Err(Error::store(format!("Can't read {}: {e}", path.display()))),
        }
    }

    /// The text of an item's file, exactly as it is on disk. Fails with
    /// exit 1 if there's no such item.
    pub fn item_text(&self, id: &str) -> Result<String> {
        let path = self.item_path(id)?;
        match fs::read_to_string(&path) {
            Ok(text) => Ok(text),
            Err(e) if e.kind() == ErrorKind::NotFound => {
                Err(Error::input(format!("There's no item {id}.")))
            }
            Err(e) => Err(Error::store(format!("Can't read {}: {e}", path.display()))),
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
    /// version would lose or change by rewriting it, such as a field a newer
    /// tracker added. Writing it back would drop that field without a word.
    pub fn update_item(&self, item: &Item) -> Result<()> {
        let path = self.item_path(&item.id)?;
        if let Err(e) = fs::metadata(&path)
            && e.kind() == ErrorKind::NotFound
        {
            return Err(Error::input(format!("There's no item {}.", item.id)));
        }
        let text = read_file(&path)?;
        let lost = item::lost_on_rewrite(&text)
            .map_err(|e| Error::store(format!("Can't parse {}: {e}", path.display())))?;
        if !lost.is_empty() {
            return Err(Error::store(format!(
                "{} has fields this version of tracker would lose or change by rewriting it: \
                 {}. It was probably written by a newer tracker, so use that one. Nothing was \
                 written.",
                path.display(),
                lost.fields().join(", ")
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
        let path = self.item_path(id)?;
        match fs::metadata(&path) {
            Err(e) if e.kind() == ErrorKind::NotFound => {
                Err(Error::input(format!("There's no item {id}.")))
            }
            _ => read_item(&path),
        }
    }

    /// Reads every item file, sorted by project and then by number.
    pub fn load_items(&self) -> Result<Vec<Item>> {
        let dir = self.items_dir();
        let entries = fs::read_dir(&dir)
            .map_err(|e| Error::store(format!("Can't read {}: {e}", dir.display())))?;

        let mut items = Vec::new();
        for entry in entries {
            let path = entry
                .map_err(|e| Error::store(format!("Can't read {}: {e}", dir.display())))?
                .path();
            // Skips anything that isn't an item file, such as `XX-1.json.4242.tmp`.
            if path.extension().and_then(|ext| ext.to_str()) != Some("json") {
                continue;
            }
            items.push(read_item(&path)?);
        }
        items.sort_by(|a, b| (&a.project, a.number()).cmp(&(&b.project, b.number())));
        Ok(items)
    }
}

/// Reads an item file and checks that it is the item its name says: the
/// file `XX-1.json` must hold `"id": "XX-1"` in project `XX`. A file copied
/// or renamed by hand would otherwise list one ID twice, and a later write
/// to `item_path(&item.id)` would overwrite the other item's file.
fn read_item(path: &Path) -> Result<Item> {
    parse_item(path, &read_file(path)?)
}

/// Parses `text`, read from `path`, with `read_item`'s checks.
fn parse_item(path: &Path, text: &str) -> Result<Item> {
    let item = item::from_json(text)
        .map_err(|e| Error::store(format!("Can't parse {}: {e}", path.display())))?;
    let named = path
        .file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or_default();
    if !item::is_valid_id(named) {
        return Err(Error::store(format!(
            "{} isn't named after an item ID, such as BM-12.json. Move it out of items/.",
            path.display()
        )));
    }
    if item.id != named {
        return Err(Error::store(format!(
            "{} holds the item \"{}\", but its name says {named}. Rename the file or fix its \
             \"id\", and check that no other file holds the same ID.",
            path.display(),
            item.id
        )));
    }
    let key = named
        .split_once('-')
        .map(|(key, _)| key)
        .unwrap_or_default();
    if item.project != key {
        return Err(Error::store(format!(
            "{} is in project \"{}\", but its ID {named} says {key}. Fix its \"project\".",
            path.display(),
            item.project
        )));
    }
    Ok(item)
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
        Err(e) => Err(Error::store(format!("Can't read {}: {e}", path.display()))),
    }
}

pub fn read_file(path: &Path) -> Result<String> {
    fs::read_to_string(path)
        .map_err(|e| Error::store(format!("Can't read {}: {e}", path.display())))
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
        Err(e) => return Err(Error::store(format!("Can't write {}: {e}", tmp.display()))),
    };
    // Flushed to disk before the rename. Otherwise a crash could keep the
    // rename but lose the data, leaving an empty file in place of the old one.
    let written = file
        .write_all(contents.as_bytes())
        .and_then(|()| file.sync_all());
    if let Err(e) = written {
        let _ = fs::remove_file(&tmp);
        return Err(Error::store(format!("Can't write {}: {e}", tmp.display())));
    }
    if let Err(e) = fs::rename(&tmp, path) {
        let _ = fs::remove_file(&tmp);
        return Err(Error::store(format!("Can't write {}: {e}", path.display())));
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
