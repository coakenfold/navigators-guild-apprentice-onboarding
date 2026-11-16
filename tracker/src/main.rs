//! The `tracker` command line (§3).

use std::io::{ErrorKind, Write};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use clap::{Parser, Subcommand};
use serde_json::{Value, json};

use tracker::config::{self, Project};
use tracker::error::{Error, Result};
use tracker::item::{self, Item, Kind, Priority, Status};
use tracker::store::Store;

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
                let _ = err.print();
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
    });

    match result {
        Ok(output) => {
            let text = if cli.json {
                format!("{}\n", pretty(&output.json))
            } else {
                output.text
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
        print_out(&format!("{}\n", pretty(&object)), code)
    } else {
        // Ignored if stderr is closed: there's nowhere left to report it.
        let _ = writeln!(std::io::stderr(), "error: {}", err.message);
        code
    }
}

fn pretty(value: &Value) -> String {
    serde_json::to_string_pretty(value).expect("JSON values always serialize")
}

fn init(store: &Store) -> Result<Output> {
    let created = store.init()?;
    let home = store.home.display();
    let text = if created {
        format!("Created the store at {home}.\n")
    } else {
        format!("The store at {home} already exists. Nothing changed.\n")
    };
    Ok(Output {
        text,
        json: json!({ "home": store.home, "created": created }),
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

    let mut projects = Vec::new();
    let mut rows = Vec::new();
    for (key, project) in &config.projects {
        let mut counts = serde_json::Map::new();
        let mut row = vec![key.clone(), project.name.clone()];
        for status in Status::ALL {
            let count = items
                .iter()
                .filter(|item| &item.project == key && item.status == status)
                .count();
            counts.insert(status.as_str().to_string(), json!(count));
            row.push(count.to_string());
        }
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

    let text = if rows.is_empty() {
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

    Ok(Output {
        text,
        json: json!({ "projects": projects }),
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
        description,
    };
    store.save_item(&item)?;

    Ok(Output {
        text: format!("Created {}: {}\n", item.id, item.title),
        json: json!({ "item": item }),
    })
}

/// Refuses text that's printed on one line, such as a title, if it holds a
/// line break or a character that changes what the terminal shows: control
/// characters, such as the escape that starts a colour code or moves the
/// cursor, and the Unicode marks that reorder text. Otherwise an agent's
/// title could rewrite the lines I read before deciding on its work.
/// `what` names the text, and `fix` says what to do instead.
fn check_printable(text: &str, what: &str, fix: &str) -> Result<()> {
    for c in text.chars() {
        let line_break = matches!(
            c,
            '\n' | '\r' | '\u{0b}' | '\u{0c}' | '\u{85}' | '\u{2028}' | '\u{2029}'
        );
        if line_break {
            return Err(Error::input(format!("{what} must fit on one line. {fix}")));
        }
        let reorders = matches!(c, '\u{200e}' | '\u{200f}' | '\u{202a}'..='\u{202e}' | '\u{2066}'..='\u{2069}');
        if c.is_control() || reorders {
            return Err(Error::input(format!(
                "{what} can't contain the control character U+{:04X}, which would change \
                 what the terminal shows. {fix}",
                c as u32
            )));
        }
    }
    Ok(())
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

        match config.projects.get(project) {
            Some(p) => text.push_str(&format!("{project}  {}\n", p.name)),
            None => text.push_str(&format!("{project}\n")),
        }
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

/// Lines up rows of cells into columns two spaces apart. The last column
/// isn't padded.
fn table(rows: &[Vec<String>]) -> String {
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
