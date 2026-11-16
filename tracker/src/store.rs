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
    /// alone. Returns true if the store is new.
    pub fn init(&self) -> Result<bool> {
        create_private_dir(&self.home)
            .map_err(|e| Error::store(format!("Can't create {}: {e}", self.home.display())))?;
        let items = self.items_dir();
        fs::create_dir_all(&items)
            .map_err(|e| Error::store(format!("Can't create {}: {e}", items.display())))?;
        let _lock = self.lock()?;
        let created = !self.config_path().exists();
        if created {
            write_file(&self.config_path(), "{\n  \"projects\": {}\n}\n")?;
        }
        if !self.counters_path().exists() {
            write_file(&self.counters_path(), "{}\n")?;
        }
        Ok(created)
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

    /// Hands out the next ID for `project` from `counters.json` and records
    /// the number after it. IDs never come from the item files, so a deleted
    /// item's ID is never reused. Call this only while holding the lock.
    ///
    /// The item files are a tripwire, though: a counter at or below a number
    /// already on disk means `counters.json` was lost or reset, and handing
    /// out its next number could reuse a deleted item's ID, whose scoped
    /// commits would then count as the new item's work (§2.8).
    pub fn next_id(&self, project: &str) -> Result<String> {
        let path = self.counters_path();
        let text = read_file(&path)?;
        let mut counters: BTreeMap<String, u64> = serde_json::from_str(&text)
            .map_err(|e| Error::store(format!("Can't parse {}: {e}", path.display())))?;

        let number = counters.get(project).copied().unwrap_or(1);
        if let Some(highest) = self.highest_number_on_disk(project)?
            && number <= highest
        {
            return Err(Error::store(format!(
                "{} says the next {project} item is {project}-{number}, but {project}-{highest} \
                 already exists, so the counter was probably lost or reset. Set \"{project}\" in \
                 counters.json above the highest {project} number ever used: at least {}.",
                path.display(),
                highest + 1
            )));
        }

        counters.insert(project.to_string(), number + 1);
        let text = serde_json::to_string_pretty(&counters).expect("counters always serialize");
        write_file(&path, &format!("{text}\n"))?;
        Ok(format!("{project}-{number}"))
    }

    /// The highest number among `project`'s item files, such as 7 when
    /// `BM-7.json` is the highest, or `None` if it has none.
    fn highest_number_on_disk(&self, project: &str) -> Result<Option<u64>> {
        let dir = self.items_dir();
        let entries = fs::read_dir(&dir)
            .map_err(|e| Error::store(format!("Can't read {}: {e}", dir.display())))?;
        let prefix = format!("{project}-");
        let mut highest = None;
        for entry in entries {
            let entry =
                entry.map_err(|e| Error::store(format!("Can't read {}: {e}", dir.display())))?;
            let name = entry.file_name();
            let number = name
                .to_str()
                .and_then(|name| name.strip_suffix(".json"))
                .and_then(|name| name.strip_prefix(&prefix))
                .and_then(|n| n.parse::<u64>().ok());
            if let Some(number) = number {
                highest = highest.max(Some(number));
            }
        }
        Ok(highest)
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
                lost.join(", ")
            )));
        }
        write_file(&path, &item::to_json(item))
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
            // Skips anything that isn't an item file, such as `XX-1.json.tmp`.
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
    let text = read_file(path)?;
    let item = item::from_json(&text)
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

pub fn read_file(path: &Path) -> Result<String> {
    fs::read_to_string(path)
        .map_err(|e| Error::store(format!("Can't read {}: {e}", path.display())))
}

/// Writes `contents` to a temporary file beside `path`, then renames it over
/// `path`, so nobody ever reads a half-written file (§4.4).
pub fn write_file(path: &Path, contents: &str) -> Result<()> {
    let mut tmp_name = path.file_name().unwrap_or_default().to_os_string();
    tmp_name.push(".tmp");
    let tmp = path.with_file_name(tmp_name);

    // Flushed to disk before the rename. Otherwise a crash could keep the
    // rename but lose the data, leaving an empty file in place of the old one.
    let written = File::create(&tmp).and_then(|mut file| {
        file.write_all(contents.as_bytes())?;
        file.sync_all()
    });
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
