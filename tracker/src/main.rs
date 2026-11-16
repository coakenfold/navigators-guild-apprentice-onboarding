//! The `tracker` command line (§3).

use std::io::{ErrorKind, IsTerminal, Write};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use clap::builder::PossibleValuesParser;
use clap::{Parser, Subcommand};
use serde_json::{Value, json};

use tracker::config::{self, Project};
use tracker::error::{Error, Result};
use tracker::item::{
    self, HistoryEntry, Item, Kind, Priority, Status, Verification, VerifiedBy, Via,
};
use tracker::states;
use tracker::store::Store;
use tracker::text::{self, check_printable};

#[derive(Parser)]
#[command(name = "tracker", version, about = "A verification-gated work tracker")]
struct Cli {
    /// Print one JSON object instead of text
    #[arg(long, global = true)]
    json: bool,

    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Create the store (`$TRACKER_HOME`, or ~/.tracker) if it doesn't exist
    Init,
    /// Register projects and list them
    Project {
        #[command(subcommand)]
        command: ProjectCommand,
    },
    /// Create an open item
    Create {
        /// What the item is, on one line
        title: String,
        /// Free markdown, stored in the item's `description` field
        #[arg(long)]
        description: Option<String>,
        /// The project's key. Defaults to the project whose root holds the current folder
        #[arg(long)]
        project: Option<String>,
    },
    /// List items, grouped by project
    List,
    /// Start work on an open item: open → in-progress
    Start {
        /// The item's ID, such as BM-12
        id: String,
    },
    /// Submit an in-progress item: → done
    ///
    /// Until the gates exist, it goes straight to done, with a verification
    /// record that says it's unchecked
    Submit {
        /// The item's ID, such as BM-12
        id: String,
    },
    /// Put an open or in-progress item back in the backlog: → shelved
    Shelve {
        /// The item's ID, such as BM-12
        id: String,
        /// Why, kept in the item's history. Required
        #[arg(long)]
        reason: Option<String>,
    },
    /// Bring a done or shelved item back: → open
    Reopen {
        /// The item's ID, such as BM-12
        id: String,
        /// Why, kept in the item's history. Required
        #[arg(long)]
        reason: Option<String>,
    },
    /// Move an item to a state, through the command that makes the move
    ///
    /// in-progress runs start, done runs submit, shelved runs shelve and
    /// open runs reopen. An item in review can't be moved this way
    Status {
        /// The item's ID, such as BM-12
        id: String,
        /// The state to move it to
        #[arg(value_parser = PossibleValuesParser::new(Status::ALL.map(Status::as_str)))]
        state: String,
        /// Why, for a move to open or shelved, which needs one
        #[arg(long)]
        reason: Option<String>,
    },
}

#[derive(Subcommand)]
enum ProjectCommand {
    /// Register a project
    Add {
        /// The key that starts each item ID, such as BM for BM-12
        key: String,
        /// A readable name for the project
        #[arg(long)]
        name: String,
        /// The project's folder: a repository's top, or a folder inside one
        #[arg(long)]
        root: PathBuf,
        /// The project's design doc, relative to its root
        #[arg(long)]
        design: Option<String>,
    },
    /// List projects with item counts by state
    List,
}

/// What a command prints: `text` normally, or `json` with `--json`.
struct Output {
    text: String,
    json: Value,
}

fn main() -> ExitCode {
    let cli = match Cli::try_parse() {
        Ok(cli) => cli,
        Err(err) if !err.use_stderr() => {
            // --help and --version.
            let _ = err.print();
            return ExitCode::SUCCESS;
        }
        Err(err) => {
            // Bad arguments are bad input, so exit 1 rather than clap's 2,
            // which here means a gate refused (§3).
            // `args_os`, because `args` panics on an argument that isn't
            // valid UTF-8, which is one of the errors being reported here.
            let wants_json = std::env::args_os().any(|arg| arg == "--json");
            if !wants_json {
                // clap's message repeats the bad argument, so it's escaped
                // like every other output.
                let message = text::for_terminal(&err.to_string(), true);
                let _ = write!(std::io::stderr(), "{message}");
                return ExitCode::from(1);
            }
            // clap's message without its usage and help hints, on one line.
            let message = err.to_string();
            let lines: Vec<&str> = message
                .lines()
                .take_while(|line| !line.starts_with("Usage:"))
                .map(|line| line.trim())
                .filter(|line| !line.is_empty())
                .collect();
            let message = lines.join(" ");
            let message = message.strip_prefix("error: ").unwrap_or(&message);
            return fail(&Error::input(message.to_string()), true);
        }
    };

    let result = Store::locate().and_then(|store| match cli.command {
        Command::Init => init(&store),
        Command::Project { command } => match command {
            ProjectCommand::Add {
                key,
                name,
                root,
                design,
            } => project_add(&store, &key, &name, &root, design),
            ProjectCommand::List => project_list(&store),
        },
        Command::Create {
            title,
            description,
            project,
        } => create(&store, &title, description, project),
        Command::List => list(&store),
        Command::Start { id } => move_item(&store, &id, Via::Start, None),
        Command::Submit { id } => move_item(&store, &id, Via::Submit, None),
        Command::Shelve { id, reason } => move_item(&store, &id, Via::Shelve, reason),
        Command::Reopen { id, reason } => move_item(&store, &id, Via::Reopen, reason),
        Command::Status { id, state, reason } => {
            let state = Status::parse(&state).expect("clap only accepts state names");
            status(&store, &id, state, reason)
        }
    });

    match result {
        Ok(output) => {
            let text = if cli.json {
                json_text(&output.json)
            } else {
                text::for_terminal(&output.text, true)
            };
            print_out(&text, ExitCode::SUCCESS)
        }
        Err(err) => fail(&err, cli.json),
    }
}

/// Writes `text` to stdout and returns `code`. A reader that has gone away,
/// as in `tracker list | head -1`, isn't an error: it just didn't want the
/// rest. `println!` would panic there instead.
fn print_out(text: &str, code: ExitCode) -> ExitCode {
    let mut stdout = std::io::stdout().lock();
    match stdout
        .write_all(text.as_bytes())
        .and_then(|()| stdout.flush())
    {
        Ok(()) => code,
        Err(e) if e.kind() == ErrorKind::BrokenPipe => code,
        Err(e) => {
            let _ = writeln!(std::io::stderr(), "error: Can't write the output: {e}");
            ExitCode::from(3)
        }
    }
}

/// Reports an error, as text on stderr or as one JSON object on stdout.
fn fail(err: &Error, json: bool) -> ExitCode {
    let code = ExitCode::from(err.code);
    if json {
        let object = json!({ "error": err.message, "exit_code": err.code });
        print_out(&json_text(&object), code)
    } else {
        // Ignored if stderr is closed: there's nowhere left to report it.
        // Messages can repeat input, such as an unknown project key, so
        // they're escaped like every other output.
        let message = text::for_terminal(&err.message, false);
        let _ = writeln!(std::io::stderr(), "error: {message}");
        code
    }
}

/// `value` as pretty-printed JSON with a final newline, with every
/// character that would change the terminal's display escaped.
fn json_text(value: &Value) -> String {
    let text = serde_json::to_string_pretty(value).expect("JSON values always serialize");
    format!("{}\n", text::for_json(&text))
}

fn init(store: &Store) -> Result<Output> {
    let init = store.init()?;
    let home = store.home.display();
    let mut text = if init.created {
        format!("Created the store at {home}.\n")
    } else if init.repaired.is_empty() {
        format!("The store at {home} already exists. Nothing changed.\n")
    } else {
        let back = if init.repaired.len() == 1 {
            "it's"
        } else {
            "they're"
        };
        format!(
            "The store at {home} was missing {}, so {back} back.\n",
            init.repaired.join(" and ")
        )
    };
    for part in &init.repaired {
        text.push_str(match *part {
            "config.json" => {
                "- config.json has no projects now. Register them again with \
                 `tracker project add`.\n"
            }
            "counters.json" => {
                "- counters.json is empty, so every project's IDs start again at 1. Set the \
                 counter of each project that has used IDs above the highest one, so no \
                 deleted item's ID is handed out again. `create` refuses while a counter is at \
                 or below a number still in items/.\n"
            }
            _ => "- items/ is empty. Any items that were in it are gone.\n",
        });
    }
    Ok(Output {
        text,
        json: json!({ "home": store.home, "created": init.created, "repaired": init.repaired }),
    })
}

fn project_add(
    store: &Store,
    key: &str,
    name: &str,
    root: &Path,
    design: Option<String>,
) -> Result<Output> {
    store.require_exists()?;
    let key = config::normalize_key(key)?;
    let name = name.trim();
    if name.is_empty() {
        return Err(Error::input(
            "A project's --name can't be empty.".to_string(),
        ));
    }
    check_printable(name, "A project's --name", "Use plain text.")?;

    let root = std::fs::canonicalize(root).map_err(|e| {
        Error::input(format!(
            "Can't use {} as a project root: {e}",
            root.display()
        ))
    })?;
    if !root.is_dir() {
        return Err(Error::input(format!(
            "{} isn't a folder, so it can't be a project root.",
            root.display()
        )));
    }
    let Some(root) = root.to_str() else {
        return Err(Error::input(format!(
            "{} isn't valid UTF-8, so it can't be stored in config.json.",
            root.display()
        )));
    };
    // `project list` prints the root, so it follows the title's rules.
    check_printable(
        root,
        "A project's root",
        "Rename the folder, or register one without such characters in its path.",
    )?;

    let project = Project {
        name: name.to_string(),
        root: root.to_string(),
        design,
    };
    let _lock = store.lock()?;
    // Two projects with one root would leave `create` guessing which one the
    // current folder means. A root nested inside another is fine: the
    // deeper one wins.
    let existing = config::load(store)?;
    if let Some(other) = config::project_with_root(&existing, Path::new(root))
        && other != key
    {
        return Err(Error::input(format!(
            "{other} already uses {root} as its root, so {key} can't share it. \
             Register {key} with a different root, or use {other}."
        )));
    }
    config::add_project(store, &key, &project)?;

    Ok(Output {
        text: format!("Registered {key} ({}) at {}.\n", project.name, project.root),
        json: json!({
            "key": key,
            "name": project.name,
            "root": project.root,
            "design": project.design,
        }),
    })
}

fn project_list(store: &Store) -> Result<Output> {
    store.require_exists()?;
    let config = config::load(store)?;
    let items = store.load_items()?;

    // A project's item counts for all five states, as JSON and as cells.
    let counts_for = |key: &str| {
        let mut counts = serde_json::Map::new();
        let mut cells = Vec::new();
        for status in Status::ALL {
            let count = items
                .iter()
                .filter(|item| item.project == key && item.status == status)
                .count();
            counts.insert(status.as_str().to_string(), json!(count));
            cells.push(count.to_string());
        }
        (counts, cells)
    };

    let mut projects = Vec::new();
    let mut rows = Vec::new();
    for (key, project) in &config.projects {
        let (counts, cells) = counts_for(key);
        let mut row = vec![key.clone(), project.name.clone()];
        row.extend(cells);
        row.push(project.root.clone());
        rows.push(row);
        projects.push(json!({
            "key": key,
            "name": project.name,
            "root": project.root,
            "design": project.design,
            "counts": counts,
        }));
    }

    // Items whose project isn't in config.json, such as one removed by
    // hand, are still counted, so `list` and `project list` agree.
    let mut unregistered_keys: Vec<&str> = items
        .iter()
        .map(|item| item.project.as_str())
        .filter(|key| !config.projects.contains_key(*key))
        .collect();
    unregistered_keys.dedup();
    let mut unregistered = Vec::new();
    for key in &unregistered_keys {
        let (counts, cells) = counts_for(key);
        let mut row = vec![key.to_string(), "(not in config.json)".to_string()];
        row.extend(cells);
        rows.push(row);
        unregistered.push(json!({ "key": key, "counts": counts }));
    }

    let mut text = if rows.is_empty() {
        "No projects yet. Register one with `tracker project add KEY --name NAME --root PATH`.\n"
            .to_string()
    } else {
        let mut header = vec!["KEY".to_string(), "NAME".to_string()];
        for status in Status::ALL {
            header.push(status.as_str().to_uppercase());
        }
        header.push("ROOT".to_string());
        rows.insert(0, header);
        table(&rows)
    };
    if !unregistered_keys.is_empty() {
        let (them, have) = if unregistered_keys.len() == 1 {
            ("it", "has")
        } else {
            ("them", "have")
        };
        text.push_str(&format!(
            "\n{} {have} items but no project in config.json. Register {them} again with \
             `tracker project add`.\n",
            unregistered_keys.join(", ")
        ));
    }

    Ok(Output {
        text,
        json: json!({ "projects": projects, "unregistered": unregistered }),
    })
}

fn create(
    store: &Store,
    title: &str,
    description: Option<String>,
    project: Option<String>,
) -> Result<Output> {
    store.require_exists()?;
    let title = title.trim();
    if title.is_empty() {
        return Err(Error::input(
            "A title can't be empty or only whitespace. Say what the item is, such as \
             tracker create \"Search filters bookmarks by tag\"."
                .to_string(),
        ));
    }
    check_printable(title, "A title", "Put the details in --description.")?;
    let description = description.filter(|text| !text.trim().is_empty());

    let _lock = store.lock()?;
    let config = config::load(store)?;
    let key = match project {
        Some(key) => {
            let key = key.trim().to_ascii_uppercase();
            if !config.projects.contains_key(&key) {
                return Err(Error::input(unknown_project(&config, &key)));
            }
            key
        }
        None => {
            let cwd = std::env::current_dir()
                .map_err(|e| Error::input(format!("Can't read the current folder: {e}")))?;
            match config::project_for_dir(&config, &cwd) {
                Some(key) => key,
                None => return Err(Error::input(no_project_here(&config))),
            }
        }
    };

    let id = store.next_id(&key)?;
    let now = item::now();
    let item = Item {
        id,
        title: title.to_string(),
        project: key,
        kind: Kind::Change,
        status: Status::Open,
        priority: Priority::Medium,
        created: now,
        updated: now,
        verification: None,
        history: Vec::new(),
        description,
    };
    store.save_item(&item)?;

    Ok(Output {
        text: format!("Created {}: {}\n", item.id, item.title),
        json: json!({ "item": item }),
    })
}

fn unknown_project(config: &config::Config, key: &str) -> String {
    if config.projects.is_empty() {
        format!(
            "There's no project {key}, and no projects yet. Register one with `tracker project add`."
        )
    } else {
        format!(
            "There's no project {key}. Registered projects: {}.",
            config::key_list(config)
        )
    }
}

fn no_project_here(config: &config::Config) -> String {
    if config.projects.is_empty() {
        "There are no projects yet. Register one with `tracker project add`.".to_string()
    } else {
        format!(
            "The current folder isn't inside any project's root. Run this from inside one, \
             or pass --project with one of: {}.",
            config::key_list(config)
        )
    }
}

fn list(store: &Store) -> Result<Output> {
    store.require_exists()?;
    let config = config::load(store)?;
    let items = store.load_items()?;

    let mut text = String::new();
    if items.is_empty() {
        text.push_str("No items yet. Create one with `tracker create \"title\"`.\n");
    }
    let mut start = 0;
    while start < items.len() {
        // `load_items` sorts by project, so each project's items sit together.
        let project = &items[start].project;
        let end = start
            + items[start..]
                .iter()
                .take_while(|item| &item.project == project)
                .count();

        let heading = match config.projects.get(project) {
            Some(p) => format!("{project}  {}", p.name),
            None => project.clone(),
        };
        text.push_str(&text::for_terminal(&heading, false));
        text.push('\n');
        let rows: Vec<Vec<String>> = items[start..end]
            .iter()
            .map(|item| {
                vec![
                    format!("  {}", item.id),
                    item.status.as_str().to_string(),
                    item.title.clone(),
                ]
            })
            .collect();
        text.push_str(&table(&rows));
        start = end;
    }

    Ok(Output {
        text,
        json: json!({ "items": items }),
    })
}

/// Runs one of the commands that move an item between states (§2.2):
/// `start`, `submit`, `shelve` or `reopen`.
fn move_item(store: &Store, id: &str, via: Via, reason: Option<String>) -> Result<Output> {
    store.require_exists()?;
    let id = normalize_id(id);
    let _lock = store.lock()?;
    let item = store.load_item(&id)?;
    let Some(step) = states::find(via, item.status) else {
        return Err(Error::input(states::not_from(&id, item.status, via)));
    };
    let reason = check_reason(&id, via, reason, &states::command(&id, via))?;
    apply(store, item, step, reason)
}

/// `status <id> <state>`: the alias that maps a state onto the command that
/// moves an item there (§3).
fn status(store: &Store, id: &str, to: Status, reason: Option<String>) -> Result<Output> {
    store.require_exists()?;
    let id = normalize_id(id);
    let _lock = store.lock()?;
    let item = store.load_item(&id)?;
    let step = match states::via_to(to) {
        // A held item is mine to decide, with approve or reject (§2.7).
        Some(_) if item.status == Status::Review => None,
        Some(via) => states::find(via, item.status),
        None => None,
    };
    let Some(step) = step else {
        return Err(Error::input(states::not_to(&id, item.status, to)));
    };

    let state = to.as_str();
    if reason.is_some() && !step.via.needs_reason() {
        return Err(Error::input(format!(
            "`status {id} {state}` runs `{}`, which takes no reason. Only a move to open or \
             shelved does. Run it without --reason.",
            step.via.as_str()
        )));
    }
    let how = format!("tracker status {id} {state} --reason \"…\"");
    let reason = check_reason(&id, step.via, reason, &how)?;
    apply(store, item, step, reason)
}

/// The reason for a move that needs one, trimmed. Refuses a missing or
/// blank reason, saying how to give one (`how`), and one that doesn't fit
/// on a line, since it's printed among other lines. `None` for a move that
/// needs no reason.
fn check_reason(id: &str, via: Via, reason: Option<String>, how: &str) -> Result<Option<String>> {
    if !via.needs_reason() {
        return Ok(None);
    }
    let reason = reason.map(|r| r.trim().to_string()).unwrap_or_default();
    if reason.is_empty() {
        let doing = match via {
            Via::Shelve => "Shelving",
            _ => "Reopening",
        };
        return Err(Error::input(format!(
            "{doing} {id} needs a reason, which is kept in its history: `{how}`."
        )));
    }
    check_printable(&reason, "A reason", "Say why in one line.")?;
    Ok(Some(reason))
}

/// Makes `step` on `item`, which was read under the lock, adds its history
/// entry, and writes the item.
fn apply(
    store: &Store,
    mut item: Item,
    step: states::Move,
    reason: Option<String>,
) -> Result<Output> {
    let now = item::now();
    item.status = step.to;
    item.updated = now;
    let mut text = format!(
        "{}: {} → {}\n",
        item.id,
        step.from.as_str(),
        step.to.as_str()
    );
    if step.via == Via::Submit {
        item.verification = Some(Verification {
            by: VerifiedBy::Unchecked,
            at: now,
            note: Some(states::UNCHECKED_NOTE.to_string()),
        });
        text.push_str(&format!("Unchecked. {}\n", states::UNCHECKED_NOTE));
    }
    item.history.push(HistoryEntry {
        at: now,
        from: step.from,
        to: step.to,
        via: step.via,
        // Only my own moves come from a terminal (§2.7).
        interactive: std::io::stdin().is_terminal(),
        reason,
    });
    store.update_item(&item)?;
    let json = if step.via == Via::Submit {
        // §3's shape for a submit. The gates (slice 6) and triggers (slice 9)
        // fill in `gates` and `holds`, and can make the result `held`.
        json!({
            "id": item.id,
            "result": "done",
            "status": item.status.as_str(),
            "gates": [],
            "holds": [],
            "item": item,
        })
    } else {
        json!({ "item": item })
    };
    Ok(Output { text, json })
}

/// An ID as typed, with its key in capitals, so `xx-1` finds XX-1.
fn normalize_id(id: &str) -> String {
    id.trim().to_ascii_uppercase()
}

/// Lines up rows of cells into columns two spaces apart. The last column
/// isn't padded. Each cell is escaped first, so a title or name edited by
/// hand can't break its line or change what the terminal shows.
fn table(rows: &[Vec<String>]) -> String {
    let rows: Vec<Vec<String>> = rows
        .iter()
        .map(|row| {
            row.iter()
                .map(|cell| text::for_terminal(cell, false))
                .collect()
        })
        .collect();
    let rows = &rows;
    let columns = rows.iter().map(|row| row.len()).max().unwrap_or(0);
    let mut widths = vec![0; columns];
    for row in rows {
        for (i, cell) in row.iter().enumerate() {
            widths[i] = widths[i].max(cell.chars().count());
        }
    }

    let mut text = String::new();
    for row in rows {
        let mut line = String::new();
        for (i, cell) in row.iter().enumerate() {
            if i + 1 == row.len() {
                line.push_str(cell);
            } else {
                line.push_str(&format!("{cell:<width$}  ", width = widths[i]));
            }
        }
        text.push_str(line.trim_end());
        text.push('\n');
    }
    text
}
