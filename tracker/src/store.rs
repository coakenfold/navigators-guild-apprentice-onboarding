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

    pub fn item_path(&self, id: &str) -> PathBuf {
        self.items_dir().join(format!("{id}.json"))
    }

    /// Creates whatever parts of the store are missing and leaves the rest
    /// alone. Returns true if the store is new.
    pub fn init(&self) -> Result<bool> {
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

    /// Writes a new item's file. Call this only while holding the lock.
    pub fn save_item(&self, item: &Item) -> Result<()> {
        write_file(&self.item_path(&item.id), &item::to_json(item))
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
            let text = read_file(&path)?;
            let item = item::from_json(&text)
                .map_err(|e| Error::store(format!("Can't parse {}: {e}", path.display())))?;
            items.push(item);
        }
        items.sort_by_key(|item| (item.project.clone(), item.number()));
        Ok(items)
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
