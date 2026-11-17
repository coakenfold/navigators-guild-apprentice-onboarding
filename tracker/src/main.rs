//! The `tracker` command line (§3).

use std::io::{ErrorKind, Write};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use anstyle::Style;
use clap::builder::PossibleValuesParser;
use clap::{Args, Parser, Subcommand};
use serde_json::{Value, json};
use unicode_width::UnicodeWidthStr;

use tracker::config::{self, Project};
use tracker::error::{Error, Result};
use tracker::item::{
    self, HistoryEntry, Item, Kind, Priority, Status, Verification, VerifiedBy, Via,
};
use tracker::states;
use tracker::store::{Skipped, Store};
use tracker::styled::{self, Styled};
use tracker::text::{self, check_printable};
use tracker::tty;

#[derive(Parser)]
#[command(
    name = "tracker",
    version,
    about = "A verification-gated work tracker",
    long_about = "A verification-gated work tracker: one central list of work items across \
                  all your projects, kept in $TRACKER_HOME, or ~/.tracker. Run `tracker init` \
                  to create it, then `tracker project add` to register a project.",
    after_help = "Exit codes: 0 success, 1 bad input or not found, 2 refused by a gate, \
                  3 the store, a file or git can't be read.\n\
                  Run `tracker <COMMAND> --help` for a command's options."
)]
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
    ///
    /// Run it again to put back any part of the store that's missing, such
    /// as counters.json. It never changes a part that's already there
    Init,
    /// Register projects and list them
    Project {
        #[command(subcommand)]
        command: ProjectCommand,
    },
    /// Create an open item
    ///
    /// It goes in the project whose root holds the current folder, unless
    /// --project names one. Its ID is the project's key and the next
    /// number, such as BM-12, and is never used again
    Create(CreateArgs),
    /// Change an item's details, such as its priority or labels
    ///
    /// Adds one entry to the item's history, naming the fields that
    /// changed. A done item can't be edited: reopen it first. An item in
    /// review can be edited only from an interactive terminal, since it's
    /// waiting for a human's decision
    Edit(EditArgs),
    /// List items, grouped by project and sorted by priority
    ///
    /// Inside a project root it lists that project's items, and outside
    /// every root, every project's. Without --status it shows open,
    /// in-progress and review items. The filters combine
    List(ListArgs),
    /// Start work on an open item: open → in-progress
    ///
    /// Later versions run the gates and each criterion's check here first
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
    // `reason` is optional to clap, so that a missing one gets tracker's
    // own message saying how to give it. The usage line shows it as needed.
    /// Put an open or in-progress item back in the backlog: → shelved
    #[command(override_usage = "tracker shelve [OPTIONS] <ID> --reason <REASON>")]
    Shelve {
        /// The item's ID, such as BM-12
        id: String,
        /// Why, kept in the item's history. Required: one line, at most 500 characters
        #[arg(long)]
        reason: Option<String>,
    },
    /// Bring a done or shelved item back: → open
    #[command(override_usage = "tracker reopen [OPTIONS] <ID> --reason <REASON>")]
    Reopen {
        /// The item's ID, such as BM-12
        id: String,
        /// Why, kept in the item's history. Required: one line, at most 500 characters
        #[arg(long)]
        reason: Option<String>,
    },
    /// Move an item to a state, through the command that makes the move
    ///
    /// in-progress runs start, done runs submit, shelved runs shelve and
    /// open runs reopen. An item in review can't be moved this way
    #[command(override_usage = "tracker status [OPTIONS] <ID> <STATE> [--reason <REASON>]")]
    Status {
        /// The item's ID, such as BM-12
        id: String,
        /// The state to move it to
        #[arg(value_parser = PossibleValuesParser::new(Status::ALL.map(Status::as_str)))]
        state: String,
        /// Why, for a move to open or shelved, which needs one: one line, at
        /// most 500 characters
        #[arg(long)]
        reason: Option<String>,
    },
    /// Show everything about one item: its fields, verification record and history
    ///
    /// With --json, the item as this version reads it, and a warning if
    /// its file holds fields a newer tracker added
    Show {
        /// The item's ID, such as BM-12
        id: String,
    },
    /// Delete an item for good. Only from an interactive terminal
    ///
    /// Says what will be lost, then asks for the ID to be typed back.
    /// Works on an item in any state. One line in deleted.json records the
    /// delete, and the ID is never used again. An agent can't delete: it
    /// shelves the item instead
    Delete {
        /// The item's ID, such as BM-12
        id: String,
    },
}

#[derive(Args)]
struct CreateArgs {
    /// What the item is, on one line of at most 200 characters
    title: String,
    /// Free markdown, stored in the item's `description` field
    #[arg(long)]
    description: Option<String>,
    /// The project's key. Defaults to the project whose root holds the current folder
    #[arg(long)]
    project: Option<String>,
    /// What kind of work it is. A capability adds something the system couldn't do before
    #[arg(long, default_value = "change", value_parser = kinds())]
    kind: String,
    /// How soon it matters. `list` shows high first
    #[arg(long, default_value = "medium", value_parser = priorities())]
    priority: String,
    /// A label, such as bug, of at most 50 characters and with no comma. Repeat it for more than
    /// one. Labels are lowercased, and each is kept once. A capital with no lower-case form, such
    /// as ℂ, is refused
    #[arg(long = "label", value_name = "LABEL")]
    labels: Vec<String>,
    /// A reference into the project's design doc, such as DESIGN.md#2.4. The file is named
    /// relative to the current folder and must be inside the project's root. It's stored
    /// relative to the root
    #[arg(long)]
    design: Option<String>,
}

#[derive(Args)]
struct EditArgs {
    /// The item's ID, such as BM-12
    id: String,
    /// A new title, on one line of at most 200 characters
    #[arg(long)]
    title: Option<String>,
    /// A new kind
    #[arg(long, value_parser = kinds())]
    kind: Option<String>,
    /// A new priority
    #[arg(long, value_parser = priorities())]
    priority: Option<String>,
    /// A label to add, such as needs-review, of at most 50 characters and with no comma. Repeat it
    /// for more than one. It's lowercased, and a capital with no lower-case form, such as ℂ, is
    /// refused
    #[arg(long = "label", value_name = "LABEL")]
    labels: Vec<String>,
    /// A label to remove. Repeat it for more than one
    #[arg(long = "unlabel", value_name = "LABEL")]
    unlabels: Vec<String>,
    /// A new reference into the design doc, or "" to remove it. The file is named relative to
    /// the current folder and must be inside the project's root
    #[arg(long)]
    design: Option<String>,
    /// A new description, which replaces the old one, or "" to remove it
    #[arg(long)]
    description: Option<String>,
}

#[derive(Args)]
struct ListArgs {
    /// Only this project's items, from anywhere
    #[arg(long, conflicts_with = "all")]
    project: Option<String>,
    /// Every project's items, even inside a project root
    #[arg(long)]
    all: bool,
    /// Only items in these states, comma-separated, such as open,shelved.
    /// The states are open, in-progress, review, done and shelved
    #[arg(long, value_delimiter = ',', value_name = "STATES", value_parser = state_name)]
    status: Vec<String>,
    /// Only items with this priority
    #[arg(long, value_parser = priorities())]
    priority: Option<String>,
    /// Only items with this label. Repeat it to keep items with every
    /// label named
    #[arg(long = "label", value_name = "LABEL")]
    labels: Vec<String>,
    /// Only items of this kind
    #[arg(long, value_parser = kinds())]
    kind: Option<String>,
    /// Only done items verified this way
    #[arg(long, value_parser = PossibleValuesParser::new(VerifiedBy::ALL.map(VerifiedBy::as_str)))]
    by: Option<String>,
}

/// One state named in `--status`, trimmed, so `open, shelved` works as well
/// as `open,shelved`. An unknown or empty one is refused with the states
/// it could be.
fn state_name(name: &str) -> std::result::Result<String, String> {
    let name = name.trim();
    match Status::parse(name) {
        Some(status) => Ok(status.as_str().to_string()),
        None => {
            let names: Vec<String> = Status::ALL.iter().map(|s| s.as_str().to_string()).collect();
            Err(format!("use {}", states::join(&names, "or")))
        }
    }
}

/// The values `--kind` takes, so that clap names them when one is wrong.
fn kinds() -> PossibleValuesParser {
    PossibleValuesParser::new(Kind::ALL.map(Kind::as_str))
}

/// The values `--priority` takes, so that clap names them when one is wrong.
fn priorities() -> PossibleValuesParser {
    PossibleValuesParser::new(Priority::ALL.map(Priority::as_str))
}

#[derive(Subcommand)]
enum ProjectCommand {
    /// Register a project
    ///
    /// Commands run inside its root use it without --project. The root can
    /// be a repository's top or a folder inside one, and must be a folder
    /// no other project uses as its root
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
    text: Styled,
    json: Value,
    /// Printed on stderr before `text`, each after `warning:`, such as an
    /// item file `list` skipped. Not printed with `--json`, whose `json`
    /// holds the same news as data.
    warnings: Vec<String>,
}

impl Output {
    /// Output with plain text and no warnings.
    fn plain(text: String, json: Value) -> Output {
        Output {
            text: text.into(),
            json,
            warnings: Vec::new(),
        }
    }
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
        Command::Create(args) => create(&store, args),
        Command::Edit(args) => edit(&store, args),
        Command::List(args) => list(&store, args),
        Command::Start { id } => move_item(&store, &id, Via::Start, None),
        Command::Submit { id } => move_item(&store, &id, Via::Submit, None),
        Command::Shelve { id, reason } => move_item(&store, &id, Via::Shelve, reason),
        Command::Reopen { id, reason } => move_item(&store, &id, Via::Reopen, reason),
        Command::Status { id, state, reason } => {
            let state = Status::parse(&state).expect("clap only accepts state names");
            status(&store, &id, state, reason)
        }
        Command::Show { id } => show(&store, &id),
        Command::Delete { id } => delete(&store, &id),
    });

    match result {
        Ok(output) => {
            let text = if cli.json {
                json_text(&output.json)
            } else {
                for warning in &output.warnings {
                    report(styled::WARNING, "warning:", warning);
                }
                output.text.render()
            };
            print_out(&text, ExitCode::SUCCESS)
        }
        Err(err) => fail(&err, cli.json),
    }
}

/// Writes `text` to stdout and returns `code`. A reader that has gone away,
/// as in `tracker list | head -1`, isn't an error: it just didn't want the
/// rest. `println!` would panic there instead.
///
/// It goes through anstream, which drops colour codes when stdout isn't a
/// terminal, so `tracker list | cat` and every test see plain text.
fn print_out(text: &str, code: ExitCode) -> ExitCode {
    let mut stdout = anstream::stdout().lock();
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
        let mut object = json!({ "error": err.message, "exit_code": err.code });
        if let Some(allowed) = &err.allowed {
            object["allowed"] = allowed.clone();
        }
        print_out(&json_text(&object), code)
    } else {
        report(styled::ERROR, "error:", &err.message);
        code
    }
}

/// Writes `message` on one line of stderr after `label`, such as `error:`,
/// in `style` when stderr is a terminal. Ignored if stderr is closed:
/// there's nowhere left to report it. Messages can repeat input, such as an
/// unknown project key, so they're escaped like every other output.
fn report(style: Style, label: &str, message: &str) {
    let mut line = Styled::new();
    line.push_styled(label, style);
    line.push(" ");
    line.push(&text::for_terminal(message, false));
    line.push("\n");
    let _ = write!(anstream::stderr(), "{}", line.render());
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
                 or below a number still in items/ or deleted.json.\n"
            }
            _ => "- items/ is empty. Any items that were in it are gone.\n",
        });
    }
    Ok(Output::plain(
        text,
        json!({ "home": store.home, "created": init.created, "repaired": init.repaired }),
    ))
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
            "A project's --name can't be empty. Give it a readable name, such as --name \
             bookmarks."
                .to_string(),
        ));
    }
    check_printable(name, "A project's --name", "Use plain text.")?;

    let root = std::fs::canonicalize(root).map_err(|e| {
        Error::input(format!(
            "Can't use {} as a project root: {e}. Check the path, or create the folder first.",
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

    Ok(Output::plain(
        format!("Registered {key} ({}) at {}.\n", project.name, project.root),
        json!({
            "key": key,
            "name": project.name,
            "root": project.root,
            "design": project.design,
        }),
    ))
}

fn project_list(store: &Store) -> Result<Output> {
    store.require_exists()?;
    let config = config::load(store)?;
    let (items, skipped) = store.load_items()?;

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

    let mut text = Styled::new();
    if rows.is_empty() {
        text.push(
            "No projects yet. Register one with `tracker project add KEY --name NAME --root PATH`.\n",
        );
    } else {
        let mut header = vec!["KEY".to_string(), "NAME".to_string()];
        for status in Status::ALL {
            header.push(status.as_str().to_uppercase());
        }
        header.push("ROOT".to_string());
        let mut cells = vec![heading_cells(&header)];
        cells.extend(rows.iter().map(|row| plain_cells(row)));
        for line in styled_table(&cells) {
            text.append(line);
        }
    }
    if !unregistered_keys.is_empty() {
        let (them, have) = if unregistered_keys.len() == 1 {
            ("it", "has")
        } else {
            ("them", "have")
        };
        text.push(&format!(
            "\n{} {have} items but no project in config.json. Register {them} again with \
             `tracker project add`.\n",
            unregistered_keys.join(", ")
        ));
    }
    if !skipped.is_empty() {
        text.push(&format!("\n{}\n", skipped_note(&skipped, "counted")));
    }

    Ok(Output {
        text,
        json: json!({
            "projects": projects,
            "unregistered": unregistered,
            "skipped": skipped_json(&skipped),
        }),
        warnings: skipped_warnings(&skipped),
    })
}

/// A heading row: each cell in bold.
fn heading_cells(row: &[String]) -> Vec<Cell> {
    row.iter()
        .map(|cell| (cell.clone(), styled::HEADING))
        .collect()
}

/// The item files `load_items` skipped, for `--json`: each one's file, the
/// parser's line and column, or null when the problem isn't the JSON's,
/// and the reason.
fn skipped_json(skipped: &[Skipped]) -> Value {
    let files: Vec<Value> = skipped
        .iter()
        .map(|file| {
            json!({
                "file": file.file.display().to_string(),
                "line": file.line,
                "column": file.column,
                "reason": file.reason,
            })
        })
        .collect();
    Value::Array(files)
}

/// One warning per skipped item file, with its name, line and reason, and
/// how to fix it (§4.4).
fn skipped_warnings(skipped: &[Skipped]) -> Vec<String> {
    skipped
        .iter()
        .map(|file| format!("Skipped {} {}", file.describe(), file.fix))
        .collect()
}

/// The line after a list that says files were skipped, so the list isn't
/// taken for the whole store when its stdout is read on its own. `left`
/// says what they weren't, such as `listed`.
fn skipped_note(skipped: &[Skipped], left: &str) -> String {
    let files = if skipped.len() == 1 {
        "1 item file can't be read as an item, so it isn't".to_string()
    } else {
        format!(
            "{} item files can't be read as items, so they aren't",
            skipped.len()
        )
    };
    format!("{files} {left}. The warnings on stderr say which, and why.")
}

fn create(store: &Store, args: CreateArgs) -> Result<Output> {
    store.require_exists()?;
    let title = check_title(&args.title)?;
    let description = args.description.filter(|text| !text.trim().is_empty());
    let kind = Kind::parse(&args.kind).expect("clap only accepts kinds");
    let priority = Priority::parse(&args.priority).expect("clap only accepts priorities");
    let mut labels = Vec::new();
    add_labels(&mut labels, &check_new_labels(&args.labels)?);
    let design = args
        .design
        .as_deref()
        .map(check_design)
        .transpose()?
        .flatten();

    let _lock = store.lock()?;
    let config = config::load(store)?;
    let key = match args.project {
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

    let design = design
        .map(|design| resolve_design(&design, &config, &key))
        .transpose()?;

    let id = store.next_id(&key)?;
    let now = item::now();
    let item = Item {
        id,
        title,
        project: key,
        kind,
        status: Status::Open,
        priority,
        labels,
        design,
        created: now,
        updated: now,
        verification: None,
        previous_verification: None,
        history: Vec::new(),
        description,
    };
    store.save_item(&item)?;

    Ok(Output::plain(
        format!("Created {}: {}\n", item.id, item.title),
        json!({ "item": item }),
    ))
}

/// The longest title `create` and `edit --title` take, in characters. A
/// title is printed in every `list` row and heads `show`, so it says what
/// the item is in a line, and the details go in its description. 200 is
/// room for a long sentence, and four times a label's limit.
const MAX_TITLE: usize = 200;

/// A title, trimmed. Refuses one that's empty, doesn't fit on a line, or
/// is longer than `MAX_TITLE`.
fn check_title(title: &str) -> Result<String> {
    let title = title.trim();
    if title.is_empty() {
        return Err(Error::input(
            "A title can't be empty or only whitespace. Say what the item is, such as \
             tracker create \"Search filters bookmarks by tag\"."
                .to_string(),
        ));
    }
    check_printable(title, "A title", "Put the details in --description.")?;
    let length = title.chars().count();
    if length > MAX_TITLE {
        return Err(Error::input(format!(
            "A title can be at most {MAX_TITLE} characters, and this one has {length}. Say what \
             the item is in a line, and put the details in --description."
        )));
    }
    Ok(title.to_string())
}

/// A label as it's stored: trimmed and lowercased (§2.1), so `Bug` and
/// `bug` are one label. Refuses one that's empty or doesn't fit on a line.
fn check_label(label: &str) -> Result<String> {
    let label = label.trim().to_lowercase();
    if label.is_empty() {
        return Err(Error::input(
            "A label can't be empty or only whitespace. Name it, such as --label bug.".to_string(),
        ));
    }
    check_printable(&label, "A label", "Use a word or two, such as --label bug.")?;
    Ok(label)
}

/// Each label checked as `check_label` does, in the order given.
fn check_labels(labels: &[String]) -> Result<Vec<String>> {
    labels.iter().map(|label| check_label(label)).collect()
}

/// The longest label `create` and `edit --label` take, in characters. A
/// label is printed in every `list` row, so a long one widens the table.
const MAX_LABEL: usize = 50;

/// Labels being added to an item: checked as `check_labels` does, each at
/// most `MAX_LABEL` characters, with no comma, since `list` prints them as
/// `[a, b]`, and with no capital that lowercasing leaves as it is. `--unlabel`
/// has none of these rules, so a label written into a file by hand can
/// still be removed.
fn check_new_labels(labels: &[String]) -> Result<Vec<String>> {
    let labels = check_labels(labels)?;
    for label in &labels {
        // A few capitals, such as ℂ, have no lower-case form, so they're
        // still capitals after `check_label`, and `item.schema.json` forbids
        // them. `is_uppercase` also covers enclosed capitals such as 🅰,
        // which the schema allows, but which read as capitals too.
        if let Some(capital) = label.chars().find(|c| c.is_uppercase()) {
            return Err(Error::input(format!(
                "A label is stored in lower case, and \"{label}\" holds {capital} (U+{:04X}), \
                 a capital with no lower-case form. Spell it with letters that have one, \
                 such as --label c.",
                capital as u32
            )));
        }
        if label.contains(',') {
            return Err(Error::input(format!(
                "A label can't hold a comma, and \"{label}\" does: `list` separates labels \
                 with commas. Give each label its own --label, such as --label a --label b."
            )));
        }
        let length = label.chars().count();
        if length > MAX_LABEL {
            return Err(Error::input(format!(
                "A label can be at most {MAX_LABEL} characters, and \"{label}\" has {length}. \
                 Use a word or two, and put the details in --description."
            )));
        }
    }
    Ok(labels)
}

/// Adds each of `new` to `labels` that isn't there yet, so each label is
/// kept once, in the order it was first given.
fn add_labels(labels: &mut Vec<String>, new: &[String]) {
    for label in new {
        if !labels.contains(label) {
            labels.push(label.clone());
        }
    }
}

/// A design reference, trimmed, or `None` for an empty one, which `edit`
/// takes as "remove it". Refuses one that doesn't fit on a line.
fn check_design(design: &str) -> Result<Option<String>> {
    let design = design.trim();
    if design.is_empty() {
        return Ok(None);
    }
    check_printable(
        design,
        "A --design reference",
        "Name a file and section, such as DESIGN.md#2.4.",
    )?;
    Ok(Some(design.to_string()))
}

/// A design reference as it's stored: the file relative to project `key`'s
/// root, with `/` between folders, then the `#section` as given (§2.1). The
/// file is named relative to the current folder, and must be inside the
/// root, so G6 never reads a file outside it (§7). The file needn't exist
/// yet: G6 checks that. The folders above it that do exist are followed
/// through any symlinks, so a symlink can't lead outside the root either,
/// but G6 must check again, since a folder can change later.
fn resolve_design(design: &str, config: &config::Config, key: &str) -> Result<String> {
    let (file, section) = match design.split_once('#') {
        Some((file, section)) => (file, Some(section)),
        None => (design, None),
    };
    if file.is_empty() {
        return Err(Error::input(format!(
            "The --design reference \"{design}\" names no file. Name the design doc and \
             section, relative to the current folder, such as DESIGN.md#2.4."
        )));
    }
    let Some(project) = config.projects.get(key) else {
        return Err(Error::input(format!(
            "{key} isn't in config.json, so there's no root to check the --design reference \
             against. Register it again with `tracker project add`."
        )));
    };
    let root = std::fs::canonicalize(project.root_path()).map_err(|e| {
        Error::input(format!(
            "Can't read {key}'s root, {}, to check the --design reference against: {e}",
            project.root
        ))
    })?;
    let cwd = std::env::current_dir()
        .and_then(std::fs::canonicalize)
        .map_err(|e| Error::input(format!("Can't read the current folder: {e}")))?;

    // `..` and `.` are taken away by hand, since the file needn't exist yet.
    let mut path = PathBuf::new();
    for part in cwd.join(file).components() {
        match part {
            std::path::Component::CurDir => {}
            std::path::Component::ParentDir => {
                path.pop();
            }
            other => path.push(other),
        }
    }
    let path = follow_symlinks(&path);
    let inside = path
        .strip_prefix(&root)
        .ok()
        .filter(|relative| !relative.as_os_str().is_empty());
    let Some(relative) = inside else {
        return Err(Error::input(format!(
            "The --design reference \"{design}\" names {}, which isn't a file inside {key}'s \
             root, {}. The file is named relative to the current folder, {}.",
            path.display(),
            root.display(),
            cwd.display()
        )));
    };
    let parts: Vec<&str> = relative
        .components()
        .map(|part| part.as_os_str().to_str().expect("built from a String"))
        .collect();
    let mut stored = parts.join("/");
    if let Some(section) = section {
        stored.push('#');
        stored.push_str(section);
    }
    Ok(stored)
}

/// `path` with its longest part that exists followed through symlinks,
/// and the rest, which doesn't exist yet, added back as it was.
fn follow_symlinks(path: &Path) -> PathBuf {
    let mut missing = Vec::new();
    let mut existing = path;
    loop {
        if let Ok(real) = std::fs::canonicalize(existing) {
            let mut path = real;
            path.extend(missing.iter().rev());
            return path;
        }
        match (existing.parent(), existing.file_name()) {
            (Some(parent), Some(name)) => {
                missing.push(name);
                existing = parent;
            }
            _ => return path.to_path_buf(),
        }
    }
}

/// `edit`: changes an item's details and adds one history entry that names
/// the fields it changed (§2.1). Refused for a done item, whose verification
/// record is about the item as it was verified, and for an item in review
/// unless it comes from an interactive terminal.
fn edit(store: &Store, args: EditArgs) -> Result<Output> {
    store.require_exists()?;
    let id = normalize_id(&args.id);
    // Every value is checked before the lock is taken, so bad input never
    // waits for another command. Only a design reference's place in the
    // project's root waits for the item, which names the project.
    let title = args.title.as_deref().map(check_title).transpose()?;
    let kind = args
        .kind
        .as_deref()
        .map(|kind| Kind::parse(kind).expect("clap only accepts kinds"));
    let priority = args
        .priority
        .as_deref()
        .map(|priority| Priority::parse(priority).expect("clap only accepts priorities"));
    let add = check_new_labels(&args.labels)?;
    let remove = check_labels(&args.unlabels)?;
    if let Some(label) = add.iter().find(|label| remove.contains(label)) {
        return Err(Error::input(format!(
            "\"{label}\" is given to both --label and --unlabel. Keep the one you mean."
        )));
    }
    let design = args.design.as_deref().map(check_design).transpose()?;
    let description = args
        .description
        .map(|text| Some(text).filter(|text| !text.trim().is_empty()));
    let nothing_given = title.is_none()
        && kind.is_none()
        && priority.is_none()
        && add.is_empty()
        && remove.is_empty()
        && design.is_none()
        && description.is_none();
    if nothing_given {
        return Err(Error::input(format!(
            "Say what to change, with at least one of --title, --kind, --priority, --label, \
             --unlabel, --design and --description, such as `tracker edit {id} --priority high`."
        )));
    }

    let _lock = store.lock()?;
    let mut item = store.load_item(&id)?;
    if item.status == Status::Done {
        return Err(Error::input(format!(
            "{id} is done, so it can't be edited: its verification record is about the item as \
             it was verified. Reopen it first with `{}`.",
            states::command(&id, Via::Reopen)
        )));
    }
    // A held item is mine to decide (§2.2), so only I can change it while
    // I do. Like the human actions, this is a convention, not security (§7).
    if item.status == Status::Review && !tty::is_interactive() {
        return Err(Error::input(format!(
            "{id} is in review, waiting for a human to approve or reject it, so it can only be \
             edited from an interactive terminal. Wait for the decision."
        )));
    }

    let design = match design {
        Some(Some(design)) => {
            let config = config::load(store)?;
            Some(Some(resolve_design(&design, &config, &item.project)?))
        }
        other => other,
    };

    // The fields that change, in the item file's order.
    let mut fields = Vec::new();
    let mut kind_before = None;
    if let Some(title) = title
        && title != item.title
    {
        item.title = title;
        fields.push("title");
    }
    if let Some(kind) = kind
        && kind != item.kind
    {
        kind_before = Some(item.kind);
        item.kind = kind;
        fields.push("kind");
    }
    if let Some(priority) = priority
        && priority != item.priority
    {
        item.priority = priority;
        fields.push("priority");
    }
    // A label given to --unlabel that the item doesn't have is named, so a
    // typo in one such as needs-review isn't missed.
    let mut not_found = Vec::new();
    let missing: Vec<String> = remove
        .iter()
        .filter(|label| !item.labels.contains(label))
        .cloned()
        .collect();
    add_labels(&mut not_found, &missing);
    // The labels are rebuilt only when the edit names some, through
    // add_labels, so a label a hand edit wrote twice is kept once, and
    // recorded once in labels_removed: item.schema.json wants each unique,
    // and no command rewrites a history entry.
    let mut labels_removed = Vec::new();
    if !add.is_empty() || !remove.is_empty() {
        let kept: Vec<String> = item
            .labels
            .iter()
            .filter(|label| !remove.contains(label))
            .cloned()
            .collect();
        let mut labels = Vec::new();
        add_labels(&mut labels, &kept);
        add_labels(&mut labels, &add);
        let gone: Vec<String> = item
            .labels
            .iter()
            .filter(|label| !labels.contains(label))
            .cloned()
            .collect();
        add_labels(&mut labels_removed, &gone);
        if labels != item.labels {
            item.labels = labels;
            fields.push("labels");
        }
    }
    if let Some(design) = design
        && design != item.design
    {
        item.design = design;
        fields.push("design");
    }
    if let Some(description) = description
        && description != item.description
    {
        item.description = description;
        fields.push("description");
    }

    let fields: Vec<String> = fields.into_iter().map(String::from).collect();
    let mut text = if fields.is_empty() {
        // Nothing to record, so the file isn't touched.
        format!("{id} already has those details. Nothing changed.\n")
    } else {
        let now = item::now();
        item.updated = now;
        item.history.push(HistoryEntry {
            at: now,
            from: item.status,
            to: item.status,
            via: Via::Edit,
            interactive: tty::is_interactive(),
            reason: None,
            fields: fields.clone(),
            kind_before,
            labels_removed,
        });
        store.update_item(&item)?;
        format!("Edited {id}: changed {}.\n", states::join(&fields, "and"))
    };
    if !not_found.is_empty() {
        let quoted: Vec<String> = not_found
            .iter()
            .map(|label| format!("\"{label}\""))
            .collect();
        let (noun, they) = if not_found.len() == 1 {
            ("label", "it wasn't")
        } else {
            ("labels", "they weren't")
        };
        text.push_str(&format!(
            "{id} had no {noun} {}, so {they} removed.\n",
            states::join(&quoted, "or")
        ));
    }
    Ok(Output::plain(
        text,
        json!({ "fields": fields, "labels_not_found": not_found, "item": item }),
    ))
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

/// The states plain `list` shows: the work that isn't finished or put away.
const ACTIVE: [Status; 3] = [Status::Open, Status::InProgress, Status::Review];

/// `list`: the items in one project or every project (§3), filtered,
/// grouped by project and sorted by priority, highest first.
fn list(store: &Store, args: ListArgs) -> Result<Output> {
    store.require_exists()?;
    let mut labels = Vec::new();
    add_labels(&mut labels, &check_labels(&args.labels)?);
    let config = config::load(store)?;
    let (mut items, skipped) = store.load_items()?;

    // Which project: the one named, every one, or the one whose root holds
    // the current folder.
    let project = if args.all {
        None
    } else if let Some(key) = args.project {
        let key = key.trim().to_ascii_uppercase();
        // A project removed from config.json by hand still has its items.
        let known =
            config.projects.contains_key(&key) || items.iter().any(|item| item.project == key);
        if !known {
            return Err(Error::input(unknown_project(&config, &key)));
        }
        Some(key)
    } else {
        let cwd = std::env::current_dir()
            .map_err(|e| Error::input(format!("Can't read the current folder: {e}")))?;
        config::project_for_dir(&config, &cwd)
    };
    items.retain(|item| project.as_ref().is_none_or(|key| &item.project == key));
    // Whether the project, or the store, has no items at all, for the
    // empty state.
    let none_in_scope = items.is_empty();

    // Without --status, the active states, unless --by asks for done items.
    let statuses: Vec<Status> = if !args.status.is_empty() {
        let parsed = args
            .status
            .iter()
            .map(|name| Status::parse(name).expect("clap only accepts state names"));
        let mut statuses: Vec<Status> = Vec::new();
        for status in parsed {
            if !statuses.contains(&status) {
                statuses.push(status);
            }
        }
        statuses
    } else if args.by.is_some() {
        vec![Status::Done]
    } else {
        ACTIVE.to_vec()
    };
    let priority = args
        .priority
        .as_deref()
        .map(|name| Priority::parse(name).expect("clap only accepts priorities"));
    let kind = args
        .kind
        .as_deref()
        .map(|name| Kind::parse(name).expect("clap only accepts kinds"));
    let by = args
        .by
        .as_deref()
        .map(|name| VerifiedBy::parse(name).expect("clap only accepts ways of verifying"));

    items.retain(|item| {
        statuses.contains(&item.status)
            && priority.is_none_or(|priority| item.priority == priority)
            && kind.is_none_or(|kind| item.kind == kind)
            && labels.iter().all(|label| item.labels.contains(label))
            // A file written by slice 2 can hold a verification record on
            // a shelved item, so the state is checked too.
            && by.is_none_or(|by| {
                item.status == Status::Done
                    && item.verification.as_ref().is_some_and(|record| record.by == by)
            })
    });
    // `load_items` sorted by project and number, so within each priority
    // the items stay in number order.
    items.sort_by_key(|item| (item.project.clone(), item.priority.rank(), item.number()));

    let mut text = Styled::new();
    if items.is_empty() {
        let only_defaults = args.status.is_empty()
            && by.is_none()
            && priority.is_none()
            && kind.is_none()
            && labels.is_empty();
        text.push(&empty_list(
            &config,
            project.as_deref(),
            none_in_scope,
            only_defaults,
        ));
    } else {
        text.append(list_table(&items, &config));
    }

    // What the list covered, so a filtered list is never mistaken for an
    // empty store (§3).
    text.push("\n");
    match &project {
        Some(key) => text.push(&format!("{key} only. `--all` for every project.\n")),
        None => text.push("Every project.\n"),
    }
    let mut filters = Vec::new();
    if !args.status.is_empty() {
        let names: Vec<String> = statuses.iter().map(|s| s.as_str().to_string()).collect();
        filters.push(format!("status {}", states::join(&names, "or")));
    }
    if let Some(priority) = priority {
        filters.push(format!("priority {}", priority.as_str()));
    }
    if !labels.is_empty() {
        let quoted: Vec<String> = labels.iter().map(|label| format!("\"{label}\"")).collect();
        let noun = if labels.len() == 1 { "label" } else { "labels" };
        filters.push(format!("{noun} {}", states::join(&quoted, "and")));
    }
    if let Some(kind) = kind {
        filters.push(format!("kind {}", kind.as_str()));
    }
    if let Some(by) = by {
        filters.push(format!("verified by {} (done items only)", by.as_str()));
    }
    if args.status.is_empty() && by.is_none() {
        text.push("Open, in-progress and review items. `--status done,shelved` for the others.\n");
    }
    if !filters.is_empty() {
        text.push(&format!("Filters: {}.\n", filters.join(", ")));
    }
    if !skipped.is_empty() {
        text.push(&format!("{}\n", skipped_note(&skipped, "listed")));
    }

    Ok(Output {
        text,
        json: json!({ "project": project, "items": items, "skipped": skipped_json(&skipped) }),
        warnings: skipped_warnings(&skipped),
    })
}

/// What `list` says when it has no rows, by what's missing: projects,
/// any item at all in `project` (or the store, for `None`), items in the
/// states it shows by default, when `only_defaults` says no filter was
/// given, or items that match the filters.
fn empty_list(
    config: &config::Config,
    project: Option<&str>,
    none_in_scope: bool,
    only_defaults: bool,
) -> String {
    if config.projects.is_empty() && none_in_scope {
        return "No projects yet. Register one with `tracker project add KEY --name NAME --root \
                PATH`, then create items in it with `tracker create \"title\"`.\n"
            .to_string();
    }
    if none_in_scope {
        return match project {
            Some(key) => format!(
                "No {key} items yet. Create one with `tracker create \"title\" --project {key}`.\n"
            ),
            None => "No items yet. Create one with `tracker create \"title\" --project KEY`.\n"
                .to_string(),
        };
    }
    if only_defaults {
        return "No open items. Nice work!\n".to_string();
    }
    "No items match these filters.\n".to_string()
}

/// `list`'s rows: a heading row naming each column, then each item,
/// with a line naming its project before the project's first item. The
/// rows are one table, so the columns line up across projects and with
/// the heading row.
fn list_table(items: &[Item], config: &config::Config) -> Styled {
    let mut header: Vec<String> = ["  ID", "PRIORITY", "KIND", "STATUS", "TITLE", "LABELS"]
        .map(String::from)
        .to_vec();
    if items.iter().all(|item| item.labels.is_empty()) {
        header.pop();
    }
    let mut rows = vec![heading_cells(&header)];
    for item in items {
        let labels = if item.labels.is_empty() {
            String::new()
        } else {
            format!("[{}]", item.labels.join(", "))
        };
        rows.push(vec![
            (format!("  {}", item.id), Style::new()),
            (
                item.priority.as_str().to_string(),
                styled::priority(item.priority),
            ),
            (item.kind.as_str().to_string(), Style::new()),
            (
                item.status.as_str().to_string(),
                styled::status(item.status),
            ),
            (item.title.clone(), Style::new()),
            (labels, Style::new()),
        ]);
    }

    let mut lines = styled_table(&rows).into_iter();
    let mut text = Styled::new();
    text.append(lines.next().expect("the heading row"));
    for (i, (item, line)) in items.iter().zip(lines).enumerate() {
        if i == 0 || items[i - 1].project != item.project {
            let heading = match config.projects.get(&item.project) {
                Some(p) => format!("{}  {}", item.project, p.name),
                None => item.project.clone(),
            };
            text.push_styled(&text::for_terminal(&heading, false), styled::HEADING);
            text.push("\n");
        }
        text.append(line);
    }
    text
}

/// Runs one of the commands that move an item between states (§2.2):
/// `start`, `submit`, `shelve` or `reopen`.
fn move_item(store: &Store, id: &str, via: Via, reason: Option<String>) -> Result<Output> {
    store.require_exists()?;
    let id = normalize_id(id);
    let _lock = store.lock()?;
    let item = store.load_item(&id)?;
    let Some(step) = states::find(via, item.status) else {
        return Err(states::not_from(&id, item.status, via));
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
        return Err(states::not_to(&id, item.status, to));
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

/// The longest reason a move takes, in characters. A reason is printed on
/// one line among others, so the details belong in the item's description.
const MAX_REASON: usize = 500;

/// The reason for a move that needs one, trimmed. Refuses a missing or
/// blank reason, saying how to give one (`how`), one that doesn't fit on a
/// line, since it's printed among other lines, and one longer than
/// `MAX_REASON`. `None` for a move that needs no reason.
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
    let length = reason.chars().count();
    if length > MAX_REASON {
        return Err(Error::input(format!(
            "A reason can be at most {MAX_REASON} characters, and this one has {length}. Say \
             why in a sentence, and put the details in the item's description."
        )));
    }
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
    let mut text = Styled::new();
    text.push(&format!("{}: {} → ", item.id, step.from.as_str()));
    text.push_styled(step.to.as_str(), styled::status(step.to));
    text.push("\n");
    // `verification` is only present while the item is done (§2.1). A file
    // written by slice 2 can still hold one on a shelved item, so this
    // doesn't check that the move starts from done.
    if step.via == Via::Reopen
        && let Some(record) = item.verification.take()
    {
        item.previous_verification = Some(record);
    }
    if step.via == Via::Submit {
        item.verification = Some(Verification {
            by: VerifiedBy::Unchecked,
            at: now,
            note: Some(states::UNCHECKED_NOTE.to_string()),
        });
        text.push(&format!("Unchecked. {}\n", states::UNCHECKED_NOTE));
    }
    item.history.push(HistoryEntry {
        at: now,
        from: step.from,
        to: step.to,
        via: step.via,
        interactive: tty::is_interactive(),
        reason,
        fields: Vec::new(),
        kind_before: None,
        labels_removed: Vec::new(),
    });
    store.update_item(&item)?;
    let json = if step.via == Via::Submit {
        // §3's shape for a submit. The gates (slice 6) fill in `gates`, and
        // the triggers (slice 9) fill in `triggers` and `holds` and can make
        // the result `held`.
        json!({
            "id": item.id,
            "result": "done",
            "status": item.status.as_str(),
            "gates": [],
            "triggers": [],
            "holds": [],
            "item": item,
        })
    } else {
        json!({ "item": item })
    };
    Ok(Output {
        text,
        json,
        warnings: Vec::new(),
    })
}

/// `show`: everything about one item (§3): every field, its verification
/// records, and its history with the time of each entry. Free text is
/// escaped, since a file edited by hand can hold anything.
///
/// It shows the item as this version reads it. A file from a newer tracker
/// can hold fields this version doesn't know, so `show` names them, in the
/// text and as `warning` beside `item`, rather than leave them out unsaid.
fn show(store: &Store, id: &str) -> Result<Output> {
    store.require_exists()?;
    let id = normalize_id(id);
    let (item, file) = store.load_item_with_text(&id)?;
    let unknown = unknown_fields(&file);
    let config = config::load(store)?;

    let project = match config.projects.get(&item.project) {
        Some(project) => format!("{} ({})", item.project, project.name),
        None => format!("{} (not in config.json)", item.project),
    };
    let field = |name: &str, value: String, style: Style| {
        vec![(format!("  {name}"), Style::new()), (value, style)]
    };
    let mut rows = vec![
        field("project", project, Style::new()),
        field("kind", item.kind.as_str().to_string(), Style::new()),
        field(
            "status",
            item.status.as_str().to_string(),
            styled::status(item.status),
        ),
        field(
            "priority",
            item.priority.as_str().to_string(),
            styled::priority(item.priority),
        ),
    ];
    if !item.labels.is_empty() {
        rows.push(field("labels", item.labels.join(", "), Style::new()));
    }
    if let Some(design) = &item.design {
        rows.push(field("design", design.clone(), Style::new()));
    }
    rows.push(field("created", item.created.to_string(), Style::new()));
    rows.push(field("updated", item.updated.to_string(), Style::new()));

    let mut text = Styled::new();
    text.push_styled(
        &format!("{}  {}", item.id, text::for_terminal(&item.title, false)),
        styled::HEADING,
    );
    text.push("\n");
    for line in styled_table(&rows) {
        text.append(line);
    }

    if let Some(description) = &item.description {
        section(&mut text, "Description:", "");
        // Indented, except for blank lines, which stay empty.
        for line in text::for_terminal(description, true).lines() {
            if !line.is_empty() {
                text.push("  ");
            }
            text.push(line);
            text.push("\n");
        }
    }

    match &item.verification {
        Some(record) => {
            section(&mut text, "Verification:", "");
            text.push(&verification_rows(record));
        }
        None => section(&mut text, "Verification:", " none."),
    }
    if let Some(record) = &item.previous_verification {
        section(
            &mut text,
            "Previous verification,",
            " from before it was last reopened:",
        );
        text.push(&verification_rows(record));
    }

    if item.history.is_empty() {
        section(&mut text, "History:", " none yet.");
    } else {
        section(
            &mut text,
            "History,",
            &format!(" oldest first ({}):", entries(item.history.len())),
        );
        let rows: Vec<Vec<String>> = item
            .history
            .iter()
            .map(|entry| {
                let moved = if entry.from == entry.to {
                    format!("stays {}", entry.from.as_str())
                } else {
                    format!("{} → {}", entry.from.as_str(), entry.to.as_str())
                };
                let source = if entry.interactive {
                    "from a terminal"
                } else {
                    "not from a terminal"
                };
                vec![
                    format!("  {}", entry.at),
                    entry.via.as_str().to_string(),
                    moved,
                    source.to_string(),
                ]
            })
            .collect();
        for (entry, line) in item.history.iter().zip(table(&rows).lines()) {
            text.push(line);
            text.push("\n");
            let mut details = Vec::new();
            if let Some(reason) = &entry.reason {
                details.push(format!("reason: {reason}"));
            }
            if !entry.fields.is_empty() {
                details.push(format!("changed: {}", entry.fields.join(", ")));
            }
            if let Some(kind) = entry.kind_before {
                details.push(format!("kind before: {}", kind.as_str()));
            }
            if !entry.labels_removed.is_empty() {
                details.push(format!(
                    "labels removed: {}",
                    entry.labels_removed.join(", ")
                ));
            }
            for detail in details {
                text.push(&format!("      {}\n", text::for_terminal(&detail, false)));
            }
        }
    }

    let mut json = json!({ "item": item });
    if !unknown.is_empty() {
        let warning = format!(
            "{id}'s file has fields this version of tracker doesn't know, or can't show in \
             full: {}. It was probably written by a newer tracker, so use that one to see them.",
            unknown.join(", ")
        );
        text.push(&format!("\n{}\n", text::for_terminal(&warning, false)));
        json["warning"] = json!(warning);
    }
    Ok(Output {
        text,
        json,
        warnings: Vec::new(),
    })
}

/// Starts a section of `show`: a blank line, then `heading` in bold and
/// `rest` plain, on one line.
fn section(text: &mut Styled, heading: &str, rest: &str) {
    text.push("\n");
    text.push_styled(heading, styled::HEADING);
    text.push(rest);
    text.push("\n");
}

/// The top-level fields of an item file that this version doesn't know, or
/// would read as something else, such as a field a newer tracker added or a
/// history entry made by a command it doesn't have. A value that's only
/// written differently, such as `"labels": []` or a time ending `+00:00`,
/// comes from a hand edit and hides nothing, so it isn't counted. `text`
/// has already been parsed as an item, so it parses again here.
fn unknown_fields(text: &str) -> Vec<String> {
    item::lost_on_rewrite(text)
        .map(|lost| lost.unknown)
        .unwrap_or_default()
}

/// A verification record's fields as indented rows.
fn verification_rows(record: &Verification) -> String {
    let mut rows = vec![
        vec!["  by".to_string(), record.by.as_str().to_string()],
        vec!["  at".to_string(), record.at.to_string()],
    ];
    if let Some(note) = &record.note {
        rows.push(vec!["  note".to_string(), note.clone()]);
    }
    table(&rows)
}

/// "1 entry" or "6 entries".
fn entries(count: usize) -> String {
    if count == 1 {
        "1 entry".to_string()
    } else {
        format!("{count} entries")
    }
}

/// `delete`: removes an item for good, from an interactive terminal only,
/// after saying what will be lost and asking for the ID (§2.8). One line
/// in `deleted.json` records it, so the digest can list it (§2.6).
fn delete(store: &Store, id: &str) -> Result<Output> {
    store.require_exists()?;
    let id = normalize_id(id);
    let (item, before) = store.load_item_with_text(&id)?;
    // A newer tracker's file can hold more than this version would say is
    // lost, or know to clean up, such as a git ref. A delete can't be
    // undone, so it's left to the tracker that wrote the file, as
    // `Store::update_item` does for a rewrite.
    let unknown = unknown_fields(&before);
    if !unknown.is_empty() {
        return Err(Error::store(format!(
            "{} has fields this version of tracker doesn't know, or can't show in full: {}. \
             It was probably written by a newer tracker, which knows what deleting it must \
             remove, so use that one. Nothing was deleted.",
            store.item_path(&id)?.display(),
            unknown.join(", ")
        )));
    }
    // Shelving is the agent's way to put an item aside, but only open and
    // in-progress items can be shelved (§2.8), so only they get that advice.
    let otherwise = if states::find(Via::Shelve, item.status).is_some() {
        format!(
            "Only a human deletes items. An item that shouldn't be done is shelved instead, \
             with `{}`, and a human decides whether to delete it.",
            states::command(&id, Via::Shelve)
        )
    } else {
        format!(
            "Only a human deletes items, so a human decides whether to delete {id}, which is \
             {}.",
            item.status.phrase()
        )
    };
    let command = format!("tracker delete {id}");
    tty::confirm(&command, &id, &deletion_warning(&item), &otherwise)?;

    // The lock wasn't held while I decided, so nothing happens if the item
    // changed meanwhile (§4.4).
    let _lock = store.lock_unchanged(&id, &before, &command)?;

    // The ID, title, state, time and whether it came from a terminal, and
    // nothing else of the item (§2.8).
    let record = json!({
        "id": item.id,
        "title": item.title,
        "status": item.status.as_str(),
        "at": item::now(),
        "interactive": tty::is_interactive(),
    });
    let line = serde_json::to_string(&record).expect("JSON values always serialize");
    store.delete_item(&id, &text::for_json(&line))?;

    Ok(Output::plain(
        format!(
            "Deleted {id}. Its line in deleted.json records the delete, and {id} won't be used \
             again.\n"
        ),
        json!({ "deleted": record }),
    ))
}

/// What `delete` says before asking for the ID (§2.8): the item's state,
/// and what deleting it removes.
fn deletion_warning(item: &Item) -> String {
    let id = &item.id;
    let state = match (item.status, &item.verification) {
        (Status::Done, Some(record)) => match record.by {
            VerifiedBy::Auto => format!("done, verified by its gates at {}", record.at),
            VerifiedBy::Human => format!("done, approved by a human at {}", record.at),
            // Nothing verified it, so the warning mustn't say it was.
            VerifiedBy::Unchecked => format!("done at {}, unchecked: no gates ran", record.at),
        },
        (Status::Done, None) => "done, with no verification record".to_string(),
        (Status::InProgress, _) => "in-progress, so someone may be working on it".to_string(),
        (Status::Review, _) => "in review, waiting for a human's decision".to_string(),
        (status, _) => status.as_str().to_string(),
    };
    // Later slices add its criteria and, for a done item, its git ref.
    let mut lost = vec!["its file".to_string()];
    if !item.history.is_empty() {
        lost.push(format!("its history ({})", entries(item.history.len())));
    }
    if item.verification.is_some() {
        lost.push("its verification record".to_string());
    }
    if item.previous_verification.is_some() {
        lost.push("the verification record from before it was reopened".to_string());
    }
    format!(
        "{id} \"{}\" is {state}.\nDeleting it removes {}.\nAny commits scoped to it stay in \
         git, the ID won't be used again, and deleted.json records the delete for the digest.\n",
        text::for_terminal(&item.title, false),
        states::join(&lost, "and")
    )
}

/// An ID as typed, with its key in capitals, so `xx-1` finds XX-1.
fn normalize_id(id: &str) -> String {
    id.trim().to_ascii_uppercase()
}

/// One cell of a table: its text and its colour.
type Cell = (String, Style);

/// Plain cells, for a row with no colour.
fn plain_cells(row: &[String]) -> Vec<Cell> {
    row.iter()
        .map(|cell| (cell.clone(), Style::new()))
        .collect()
}

/// Lines up rows of cells into columns two spaces apart, and returns one
/// line per row, each ending in a newline. Empty cells at the end of a row
/// aren't printed, and the last one printed isn't padded, so no line ends
/// in spaces. Widths are counted in terminal columns, so a CJK character
/// or an emoji, which takes two, keeps its row in line. Each cell is
/// escaped first, so a title or name edited by hand can't break its line
/// or change what the terminal shows, and its colour goes round the cell,
/// never the padding, so the columns line up with colour or without.
fn styled_table(rows: &[Vec<Cell>]) -> Vec<Styled> {
    let rows: Vec<Vec<Cell>> = rows
        .iter()
        .map(|row| {
            row.iter()
                .map(|(cell, style)| (text::for_terminal(cell, false), *style))
                .collect()
        })
        .collect();
    let columns = rows.iter().map(|row| row.len()).max().unwrap_or(0);
    let mut widths = vec![0; columns];
    for row in &rows {
        for (i, (cell, _)) in row.iter().enumerate() {
            widths[i] = widths[i].max(cell.width());
        }
    }

    let mut lines = Vec::new();
    for row in &rows {
        let used = row
            .iter()
            .rposition(|(cell, _)| !cell.trim_end().is_empty())
            .map_or(0, |last| last + 1);
        let mut line = Styled::new();
        for (i, (cell, style)) in row[..used].iter().enumerate() {
            if i + 1 == used {
                line.push_styled(cell.trim_end(), *style);
            } else {
                line.push_styled(cell, *style);
                line.push(&" ".repeat(widths[i] - cell.width() + 2));
            }
        }
        line.push("\n");
        lines.push(line);
    }
    lines
}

/// `styled_table` for rows with no colour, as one piece of text.
fn table(rows: &[Vec<String>]) -> String {
    let rows: Vec<Vec<Cell>> = rows.iter().map(|row| plain_cells(row)).collect();
    styled_table(&rows).iter().map(Styled::plain).collect()
}

#[cfg(test)]
mod tests {
    use clap::CommandFactory;

    use super::Cli;

    #[test]
    fn the_command_line_is_well_formed() {
        Cli::command().debug_assert();
    }

    /// Slice 5: every command and every flag says what it's for in
    /// `--help`, so a new one without a doc comment fails here.
    #[test]
    fn every_command_and_flag_has_help_text() {
        fn check(command: &clap::Command, path: &str) {
            assert!(command.get_about().is_some(), "`{path}` has no help text");
            for arg in command.get_arguments() {
                let id = arg.get_id().as_str();
                if id == "help" || id == "version" {
                    continue;
                }
                assert!(arg.get_help().is_some(), "`{path}`'s {id} has no help text");
            }
            for sub in command.get_subcommands() {
                check(sub, &format!("{path} {}", sub.get_name()));
            }
        }
        check(&Cli::command(), "tracker");
    }
}
