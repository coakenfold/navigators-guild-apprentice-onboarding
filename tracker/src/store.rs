//! Where the store lives, its lock, and reading and writing its files
//! (§4.1, §4.4).

use std::collections::BTreeMap;
use std::fs::{self, File, OpenOptions};
use std::io::ErrorKind;
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
    /// created, and with exit 3 if the store is there but can't be read.
    pub fn require_exists(&self) -> Result<()> {
        let path = self.config_path();
        match fs::metadata(&path) {
            Ok(meta) if meta.is_file() => Ok(()),
            Ok(_) => Err(Error::store(format!(
                "{} isn't a file, so the store can't be read.",
                path.display()
            ))),
            Err(e) if e.kind() == ErrorKind::NotFound => Err(Error::input(format!(
                "There's no tracker store at {}. Run `tracker init`.",
                self.home.display()
            ))),
            Err(e) => Err(Error::store(format!(
                "Can't read the store at {}: {e}. Check that you can read and write that folder.",
                self.home.display()
            ))),
        }
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
    pub fn next_id(&self, project: &str) -> Result<String> {
        let path = self.counters_path();
        let text = read_file(&path)?;
        let mut counters: BTreeMap<String, u64> = serde_json::from_str(&text)
            .map_err(|e| Error::store(format!("Can't parse {}: {e}", path.display())))?;

        let number = counters.get(project).copied().unwrap_or(1);
        let id = format!("{project}-{number}");
        if self.item_path(&id).exists() {
            return Err(Error::store(format!(
                "{} says the next {project} item is {id}, but {} already exists. \
                 Set \"{project}\" in counters.json above the highest {project} number in use.",
                path.display(),
                self.item_path(&id).display()
            )));
        }

        counters.insert(project.to_string(), number + 1);
        let text = serde_json::to_string_pretty(&counters).expect("counters always serialize");
        write_file(&path, &format!("{text}\n"))?;
        Ok(id)
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

    fs::write(&tmp, contents)
        .map_err(|e| Error::store(format!("Can't write {}: {e}", tmp.display())))?;
    fs::rename(&tmp, path).map_err(|e| {
        let _ = fs::remove_file(&tmp);
        Error::store(format!("Can't write {}: {e}", path.display()))
    })
}
