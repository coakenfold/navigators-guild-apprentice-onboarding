//! Judge criteria (§2.3.1): yes/no questions a model answers about an
//! excerpt of the work, which G3 asks once every script gate has passed.
//!
//! The tool never calls a model itself. It builds a request, `model`,
//! `state` and `questions`, and runs the config's judge command with the
//! request on stdin, reading the answers from its stdout. Each answer is
//! kept in `judge/cache/` under the request's fingerprint, so unchanged work
//! gets the same answer every time.
//!
//! A judge reads its `inputs`, files under the project's root or `diff`
//! for the item's changes, with comments stripped unless `keep_comments`
//! says otherwise. Without `each`, the questions are asked once, about
//! every input. With it, they're asked once per match of its regex, about
//! the lines around it.

use std::collections::BTreeMap;
use std::fs;
use std::io::{Read, Write};
use std::os::unix::process::CommandExt;
use std::path::Path;
use std::process::{Command, Stdio};
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};

use regex::Regex;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

use crate::checks;
use crate::config::Config;
use crate::git;
use crate::item::{Answer, Criterion, Item, JudgeRecord, Question};
use crate::text;

/// The longest question, or meaning of yes or no, in characters, as for a
/// criterion's text.
const MAX_QUESTION: usize = 500;

/// The longest question `id`, in characters. It's part of every ref.
const MAX_ID: usize = 64;

/// How much of a judge command's stdout and stderr is read, in bytes. A
/// reply is a few hundred bytes.
const MAX_OUTPUT: u64 = 1024 * 1024;

/// A judge file, as `criterion add --judge` and `criterion set --judge`
/// read it (§2.3.1). It sets every judge field, and nothing else.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct JudgeFile {
    text: String,
    inputs: Vec<String>,
    #[serde(default)]
    each: Option<String>,
    #[serde(default)]
    context_lines: Option<usize>,
    #[serde(default)]
    keep_comments: bool,
    ask: Vec<Question>,
}

/// Reads a judge file's `text` as a criterion, checking every field but
/// whether its inputs exist and its `each` matches, which `check_inputs`
/// does. The error completes "The judge file … ".
pub fn parse_file(text: &str) -> Result<Criterion, String> {
    let file: JudgeFile = serde_json::from_str(text).map_err(|e| {
        if e.is_syntax() || e.is_eof() {
            format!("isn't valid JSON: {e}.")
        } else {
            format!("isn't a judge as DESIGN.md §2.3.1 describes one: {e}.")
        }
    })?;
    if file.inputs.is_empty() {
        return Err(
            "has no inputs. Name the files it reads, relative to the project's root, or \"diff\" \
             for the item's changes."
                .to_string(),
        );
    }
    for input in &file.inputs {
        one_line(input, "An input")?;
        if input.trim().is_empty() || Path::new(input).is_absolute() {
            return Err(format!(
                "names the input \"{input}\". An input is a file relative to the project's root, \
                 such as src/errors.rs, or \"diff\"."
            ));
        }
    }
    if let Some(each) = &file.each {
        Regex::new(each)
            .map_err(|e| format!("has an each, `{each}`, that isn't a valid regex: {e}"))?;
    }
    if file.ask.is_empty() {
        return Err("asks no questions. Give it at least one in \"ask\".".to_string());
    }
    let mut ids: Vec<&str> = Vec::new();
    for question in &file.ask {
        let id = question.id.as_str();
        let id_ok = !id.is_empty()
            && id.chars().count() <= MAX_ID
            && id
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-');
        if !id_ok {
            return Err(format!(
                "has a question whose id, \"{id}\", isn't up to {MAX_ID} letters, digits, _ and \
                 -, such as names_problem. The id is part of the question's ref."
            ));
        }
        if ids.contains(&id) {
            return Err(format!("has two questions with the id \"{id}\"."));
        }
        ids.push(id);
        for (what, value) in [
            ("question", Some(&question.ask)),
            ("yes", question.yes.as_ref()),
            ("no", question.no.as_ref()),
        ] {
            let Some(value) = value else { continue };
            let what = format!("Question {id}'s {what}");
            one_line(value, &what)?;
            if value.trim().is_empty() || value.chars().count() > MAX_QUESTION {
                return Err(format!(
                    "has a question, {id}, whose {} is empty or longer than {MAX_QUESTION} \
                     characters.",
                    if what.ends_with("question") {
                        "text"
                    } else {
                        "meaning"
                    }
                ));
            }
        }
        if question.pass_when == Some(Answer::Unsure) {
            return Err(format!(
                "has a question, {id}, that passes when unsure. Its pass_when is yes or no."
            ));
        }
    }
    Ok(Criterion {
        text: file.text,
        inputs: file.inputs,
        each: file.each,
        context_lines: file.context_lines,
        keep_comments: file.keep_comments,
        ask: file.ask,
        ..Criterion::default()
    })
}

fn one_line(value: &str, what: &str) -> Result<(), String> {
    text::check_printable(value, what, "Keep it on one line.")
        .map_err(|e| format!("is refused: {}", e.message))
}

/// Checks, when a judge is added, that each of its file inputs exists in
/// `root`, and that its `each` matches something in them, so a judge that
/// would have nothing to grade is refused up front (§2.3.1). A `diff`
/// input isn't read here: the item may have no commits yet. The error
/// completes "The judge file … ".
pub fn check_inputs(criterion: &Criterion, root: &Path, key: &str) -> Result<(), String> {
    let mut files = Vec::new();
    for input in criterion.inputs.iter().filter(|input| *input != "diff") {
        let text = read_file(root, input, key)?;
        files.push((input.clone(), strip(criterion, input, &text)));
    }
    let only_files = files.len() == criterion.inputs.len();
    if let Some(each) = &criterion.each
        && only_files
        && excerpts(criterion, &files, 0)?.is_empty()
    {
        return Err(nothing_to_grade(each, &criterion.inputs));
    }
    Ok(())
}

/// One judge question that was answered, for G3, T10 and the
/// verification record.
#[derive(Debug, Clone, PartialEq)]
pub struct Answered {
    /// The criterion, numbered from 1.
    pub criterion: usize,
    /// Its ref, such as `c2:src/import.js:57:suggests_next_step` (§2.3.2).
    pub judge_ref: String,
    /// The question, as the judge was asked it.
    pub question: String,
    pub answer: Answer,
    pub reason: String,
    /// The answer that passes.
    pub passing: Answer,
    /// The model that answered.
    pub model: String,
    /// The fingerprint of the request it was asked in.
    pub request: String,
    /// The excerpt it was asked about, as labelled sections.
    pub excerpt: String,
}

/// What an answer means for the item (§2.3.1).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
    Pass,
    /// The gate refuses.
    Fail,
    /// The item is held for a human (T10).
    Unsure,
}

impl Answered {
    pub fn outcome(&self) -> Outcome {
        if self.answer == Answer::Unsure {
            Outcome::Unsure
        } else if self.answer == self.passing {
            Outcome::Pass
        } else {
            Outcome::Fail
        }
    }

    /// The question in `gates` and `submit` output (§3).
    pub fn json(&self) -> Value {
        let result = match self.outcome() {
            Outcome::Pass => "pass",
            Outcome::Fail => "fail",
            Outcome::Unsure => "unsure",
        };
        json!({
            "ref": self.judge_ref,
            "criterion": self.criterion,
            "answer": self.answer,
            "reason": self.reason,
            "result": result,
        })
    }

    /// The question as the verification record keeps it (§4.3).
    pub fn record(&self) -> JudgeRecord {
        JudgeRecord {
            judge_ref: self.judge_ref.clone(),
            model: self.model.clone(),
            request: self.request.clone(),
            answer: self.answer,
            reason: self.reason.clone(),
        }
    }
}

/// What grading one judge criterion found: every question answered, and
/// every reason it couldn't be graded, such as a missing input or a failed
/// call, which fails G3.
#[derive(Debug, Default)]
pub struct Graded {
    pub answers: Vec<Answered>,
    pub failures: Vec<String>,
}

/// Grades criterion `n`, a judge, of `item`, whose project's root is
/// `root`, keeping answers in the `cache` folder.
pub fn grade(
    item: &Item,
    n: usize,
    criterion: &Criterion,
    config: &Config,
    root: &Path,
    cache: &Path,
) -> Graded {
    let mut graded = Graded::default();
    let what = format!("Criterion {n}'s judge");
    let mut inputs = Vec::new();
    for input in &criterion.inputs {
        let text = if input == "diff" {
            read_diff(item, root)
        } else {
            read_file(root, input, &item.project)
        };
        match text {
            Ok(text) => inputs.push((input.clone(), strip(criterion, input, &text))),
            Err(why) => {
                graded
                    .failures
                    .push(format!("{what} can't read its input: {why}"));
                return graded;
            }
        }
    }
    let context = criterion.context_lines.unwrap_or(config.context_lines());
    let excerpts = match excerpts(criterion, &inputs, context) {
        Ok(excerpts) => excerpts,
        Err(why) => {
            graded.failures.push(format!("{what} {why}"));
            return graded;
        }
    };
    if excerpts.is_empty() {
        let each = criterion.each.as_deref().unwrap_or_default();
        graded.failures.push(format!(
            "{what} {} A judge with nothing to grade fails rather than pass with no questions \
             asked.",
            nothing_to_grade(each, &criterion.inputs)
        ));
        return graded;
    }

    let command = config.judge_command();
    let model = config.judge_model();
    let max = config.max_input_bytes();
    let timeout = config
        .projects
        .get(&item.project)
        .map(|project| project.judge_timeout())
        .unwrap_or(Duration::from_secs(
            crate::config::DEFAULT_JUDGE_TIMEOUT_SECS,
        ));
    for excerpt in excerpts {
        let place = excerpt
            .at
            .as_ref()
            .map_or(String::new(), |at| format!(" at {at}"));
        let size: usize = excerpt.state.iter().map(|(_, value)| value.len()).sum();
        if size > max {
            graded.failures.push(format!(
                "{what}'s excerpt{place} is {size} bytes, more than the config's \
                 judge.max_input_bytes of {max}, so its questions weren't asked. Narrow its \
                 inputs or each, or lower its context_lines."
            ));
            continue;
        }
        let request = request(&model, &excerpt.state, &criterion.ask);
        let fingerprint = fingerprint(&command, &request);
        let reply = match cached(cache, &fingerprint, &criterion.ask) {
            Some(reply) => {
                eprintln!("Criterion {n}'s judge{place}: answered from the cache");
                Ok(reply)
            }
            None => ask(
                &command,
                &request,
                &model,
                &criterion.ask,
                timeout * criterion.ask.len() as u32,
                &format!("criterion {n}'s judge"),
                &place,
            )
            .and_then(|reply| save(cache, &fingerprint, &reply).map(|()| reply)),
        };
        let reply = match reply {
            Ok(reply) => reply,
            Err(why) => {
                // Asking again about the rest would most likely fail the
                // same way, and each call can take a while.
                graded.failures.push(format!(
                    "{what}{place} couldn't be asked: `{command}` {why} A failed call never \
                     counts as a pass."
                ));
                break;
            }
        };
        let shown = render(&excerpt.state);
        for question in &criterion.ask {
            let answer = &reply.answers[&question.id];
            let judge_ref = match &excerpt.at {
                Some(at) => format!("c{n}:{at}:{}", question.id),
                None => format!("c{n}:{}", question.id),
            };
            graded.answers.push(Answered {
                criterion: n,
                judge_ref,
                question: question.ask.clone(),
                answer: answer.answer,
                reason: answer.reason.clone(),
                passing: question.passing(),
                model: reply.model.clone(),
                request: fingerprint.clone(),
                excerpt: shown.clone(),
            });
        }
    }
    graded
}

fn nothing_to_grade(each: &str, inputs: &[String]) -> String {
    format!(
        "has nothing to grade: its each, `{each}`, matches nothing in {}, once comments are \
         stripped.",
        inputs.join(", ")
    )
}

/// Reads `input`, relative to `root`. It must be a regular file of UTF-8
/// text inside the root once symlinks are followed (§7), as G6 reads a
/// design doc.
fn read_file(root: &Path, input: &str, key: &str) -> Result<String, String> {
    let path = match fs::canonicalize(root.join(input)) {
        Ok(path) => path,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            return Err(format!(
                "names {input}, which doesn't exist in {key}'s root, {}.",
                root.display()
            ));
        }
        Err(e) => return Err(format!("can't read {input}: {e}.")),
    };
    if !path.starts_with(root) {
        return Err(format!(
            "names {input}, which leads to {}, outside {key}'s root. A judge reads only inside \
             the root.",
            path.display()
        ));
    }
    match fs::metadata(&path) {
        Ok(meta) if meta.is_file() => {}
        Ok(_) => return Err(format!("names {input}, which isn't a regular file.")),
        Err(e) => return Err(format!("can't read {input}: {e}.")),
    }
    fs::read_to_string(&path).map_err(|e| {
        if e.kind() == std::io::ErrorKind::InvalidData {
            format!("names {input}, which isn't UTF-8 text.")
        } else {
            format!("can't read {input}: {e}.")
        }
    })
}

/// The item's changes: each of its commits' patches, oldest first, as
/// `show --diff` prints them (§3).
fn read_diff(item: &Item, root: &Path) -> Result<String, String> {
    let mut shas = git::item_commits(root, &item.id).map_err(|e| e.message)?;
    shas.reverse();
    git::patches(root, &shas).map_err(|e| e.message)
}

/// `text`, read from `input`, with its comments stripped unless the
/// criterion keeps them.
fn strip(criterion: &Criterion, input: &str, text: &str) -> String {
    if criterion.keep_comments {
        text.to_string()
    } else if input == "diff" {
        strip_diff(text)
    } else {
        strip_comments(text, style_for(input))
    }
}

/// One excerpt that a judge's questions are asked about.
struct Excerpt {
    /// Where it is, `file:line` of an `each` match, or `None` for the whole
    /// input.
    at: Option<String>,
    /// Its labelled sections, in order: the request's `state`.
    state: Vec<(String, String)>,
}

/// The excerpts of `inputs`, each a name and its text: one of them all,
/// or, with `each`, one per line with a match, with `context` lines either
/// side. Empty when `each` matches nothing.
fn excerpts(
    criterion: &Criterion,
    inputs: &[(String, String)],
    context: usize,
) -> Result<Vec<Excerpt>, String> {
    let Some(each) = &criterion.each else {
        return Ok(vec![Excerpt {
            at: None,
            state: inputs.to_vec(),
        }]);
    };
    let pattern = Regex::new(each)
        .map_err(|e| format!("has an each, `{each}`, that isn't a valid regex: {e}"))?;
    let mut found = Vec::new();
    for (name, text) in inputs {
        let lines: Vec<&str> = text.split('\n').collect();
        let mut seen = Vec::new();
        for found_match in pattern.find_iter(text) {
            let line = text[..found_match.start()].matches('\n').count() + 1;
            if seen.contains(&line) {
                continue;
            }
            seen.push(line);
            let first = line.saturating_sub(context).max(1);
            let last = (line + context).min(lines.len());
            found.push(Excerpt {
                at: Some(format!("{name}:{line}")),
                state: vec![
                    (
                        "match".to_string(),
                        format!("{name}:{line}: {}", lines[line - 1].trim()),
                    ),
                    ("code".to_string(), lines[first - 1..last].join("\n")),
                ],
            });
        }
    }
    Ok(found)
}

/// The request the judge command reads (§2.3.1): the model, the excerpt
/// as `state`, and each question by its id. A question's `instructions`
/// is what it asks, and `criteria` says what yes (`true`) and no (`false`)
/// mean, as judge-eval's adapter reads them.
fn request(model: &str, state: &[(String, String)], ask: &[Question]) -> Value {
    let state: serde_json::Map<String, Value> = state
        .iter()
        .map(|(name, value)| (name.clone(), json!(value)))
        .collect();
    let mut questions = serde_json::Map::new();
    for question in ask {
        let mut asked = json!({ "instructions": question.ask });
        let mut criteria = serde_json::Map::new();
        if let Some(yes) = &question.yes {
            criteria.insert("true".to_string(), json!(yes));
        }
        if let Some(no) = &question.no {
            criteria.insert("false".to_string(), json!(no));
        }
        if !criteria.is_empty() {
            asked["criteria"] = Value::Object(criteria);
        }
        questions.insert(question.id.clone(), asked);
    }
    json!({ "model": model, "state": state, "questions": questions })
}

/// The request's fingerprint, its cache key: a SHA-256, in lower-case hex,
/// of the judge command and the request, which holds the excerpt, the
/// questions and the model.
fn fingerprint(command: &str, request: &Value) -> String {
    let value = json!({ "command": command, "request": request });
    Sha256::digest(value.to_string().as_bytes())
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

/// `state` as labelled sections, as the judge renders it (§2.3.1).
fn render(state: &[(String, String)]) -> String {
    state
        .iter()
        .map(|(name, value)| format!("[{name}]\n{value}"))
        .collect::<Vec<_>>()
        .join("\n\n")
}

/// A judge command's reply (§2.3.1), and what the cache keeps.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct Reply {
    adapter: String,
    model: String,
    answers: BTreeMap<String, ReplyAnswer>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    cost_usd: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ReplyAnswer {
    answer: Answer,
    reason: String,
}

/// The cached reply for request `fingerprint`, if there's one that answers
/// every question in `ask`.
fn cached(cache: &Path, fingerprint: &str, ask: &[Question]) -> Option<Reply> {
    let text = fs::read_to_string(cache.join(format!("{fingerprint}.json"))).ok()?;
    let reply: Reply = serde_json::from_str(&text).ok()?;
    ask.iter()
        .all(|question| reply.answers.contains_key(&question.id))
        .then_some(reply)
}

/// Keeps `reply` in the cache under `fingerprint`. It's written as every
/// store file is, through a temporary file, so a reader never sees half
/// of it.
fn save(cache: &Path, fingerprint: &str, reply: &Reply) -> Result<(), String> {
    fs::create_dir_all(cache)
        .map_err(|e| format!("answered, but {} can't be made: {e}.", cache.display()))?;
    let text = serde_json::to_string_pretty(reply).expect("replies always serialize");
    crate::store::write_file(
        &cache.join(format!("{fingerprint}.json")),
        &format!("{text}\n"),
    )
    .map_err(|e| format!("answered, but its answers can't be cached: {}", e.message))
}

/// Runs the judge `command` on `request`, stopping it after `timeout`,
/// and reads its reply, which must answer every question in `ask` with
/// yes, no or unsure, from `model`. `what` names the judge, and `place`
/// its excerpt, in the line printed before it runs. The error completes
/// "`command` … ".
fn ask(
    command: &str,
    request: &Value,
    model: &str,
    ask: &[Question],
    timeout: Duration,
    what: &str,
    place: &str,
) -> Result<Reply, String> {
    eprintln!(
        "Asking {what} {} question{}{place}: {}",
        ask.len(),
        if ask.len() == 1 { "" } else { "s" },
        text::for_terminal(&format!("`{command}`"), false)
    );
    let (stdout, stderr, status, timed_out) = run(command, &request.to_string(), timeout)?;
    if timed_out {
        return Err(format!(
            "ran longer than {}s, the project's judge.timeout_secs for each of its questions, so \
             it was stopped.",
            timeout.as_secs()
        ));
    }
    if !status.is_some_and(|status| status.success()) {
        let how = match status.and_then(|status| status.code()) {
            Some(code) => format!("exited with {code}"),
            None => "was stopped".to_string(),
        };
        let said = String::from_utf8_lossy(&stderr);
        let said = said.trim();
        let said = if said.is_empty() {
            String::new()
        } else {
            format!(": {}", text::for_terminal(said, false))
        };
        return Err(format!("{how}{said}."));
    }
    let reply: Reply = serde_json::from_slice(&stdout).map_err(|e| {
        format!(
            "replied with something that isn't a judge's answers ({e}): {}",
            text::for_terminal(String::from_utf8_lossy(&stdout).trim(), false)
        )
    })?;
    if reply.model != model {
        return Err(format!(
            "was asked for {model}, but {} answered.",
            text::for_terminal(&reply.model, false)
        ));
    }
    if let Some(missing) = ask
        .iter()
        .find(|question| !reply.answers.contains_key(&question.id))
    {
        return Err(format!(
            "replied without an answer to the question {}.",
            missing.id
        ));
    }
    Ok(reply)
}

/// A judge command's stdout, its stderr, how it ended and whether it ran
/// out of time.
type Ran = (Vec<u8>, Vec<u8>, Option<std::process::ExitStatus>, bool);

/// Runs `command` with `sh -c` in the system's temporary folder, with
/// `input` on stdin. Once it runs past `timeout`, it's stopped, with
/// everything it started.
fn run(command: &str, input: &str, timeout: Duration) -> Result<Ran, String> {
    let began = Instant::now();
    let mut child = Command::new("sh")
        .arg("-c")
        .arg(command)
        .current_dir(std::env::temp_dir())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .process_group(0)
        .spawn()
        .map_err(|e| format!("couldn't be run: {e}."))?;
    let mut stdin = child.stdin.take().expect("stdin is piped");
    let input = input.to_string();
    thread::spawn(move || {
        let _ = stdin.write_all(input.as_bytes());
    });
    let stdout = read_all(child.stdout.take().expect("stdout is piped"));
    let stderr = read_all(child.stderr.take().expect("stderr is piped"));
    let (status, timed_out) = checks::wait(&mut child, began + timeout);
    checks::stop_group(&child);
    let wait = Duration::from_secs(1);
    let stdout = stdout.recv_timeout(wait).unwrap_or_default();
    let stderr = stderr.recv_timeout(wait).unwrap_or_default();
    Ok((stdout, stderr, status, timed_out))
}

/// Reads `from` to its end, up to `MAX_OUTPUT` bytes, on a thread of its
/// own, and sends what it read.
fn read_all(from: impl Read + Send + 'static) -> mpsc::Receiver<Vec<u8>> {
    let (send, read) = mpsc::channel();
    thread::spawn(move || {
        let mut buffer = Vec::new();
        let _ = from.take(MAX_OUTPUT).read_to_end(&mut buffer);
        let _ = send.send(buffer);
    });
    read
}

/// How a file's comments are written, by its extension.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Style {
    /// No comments known, so nothing is stripped, as for markdown or JSON.
    None,
    /// `//` and `/* … */`, as in Rust, JavaScript, C and Go.
    CLike,
    /// `#`, as in Python, shell, Ruby, TOML and YAML.
    Hash,
    /// `<!-- … -->`, as in HTML and XML.
    Html,
}

/// The comment style of the file at `path`.
pub fn style_for(path: &str) -> Style {
    let name = path.rsplit('/').next().unwrap_or(path);
    let extension = name
        .rsplit_once('.')
        .map(|(_, extension)| extension.to_ascii_lowercase())
        .unwrap_or_default();
    match extension.as_str() {
        "rs" | "js" | "mjs" | "cjs" | "jsx" | "ts" | "tsx" | "c" | "h" | "cc" | "cpp" | "hpp"
        | "go" | "java" | "kt" | "swift" | "cs" | "scala" | "dart" | "css" | "scss" | "zig" => {
            Style::CLike
        }
        "py" | "sh" | "bash" | "zsh" | "rb" | "toml" | "yaml" | "yml" | "pl" | "r" => Style::Hash,
        "html" | "htm" | "xml" | "svg" | "vue" => Style::Html,
        _ if name == "Makefile" || name == "Dockerfile" => Style::Hash,
        _ => Style::None,
    }
}

/// `text` with its comments removed, in `style`. Every line break is kept,
/// so line numbers don't move, and each line loses its trailing spaces.
/// Strings are skipped, so `"http://…"` isn't cut. It's a scanner, not a
/// parser, so an unusual literal, such as a regex in JavaScript, can fool
/// it.
pub fn strip_comments(text: &str, style: Style) -> String {
    if style == Style::None {
        return text.to_string();
    }
    let chars: Vec<char> = text.chars().collect();
    let starts = |i: usize, token: &str| {
        token
            .chars()
            .enumerate()
            .all(|(k, c)| chars.get(i + k) == Some(&c))
    };
    let mut out = String::with_capacity(text.len());
    let mut quote: Option<char> = None;
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        if let Some(q) = quote {
            out.push(c);
            if c == '\\' && i + 1 < chars.len() {
                out.push(chars[i + 1]);
                i += 2;
                continue;
            }
            // A quote opened with ' in Python or shell ends with its line.
            if c == q || (c == '\n' && q == '\'') {
                quote = None;
            }
            i += 1;
            continue;
        }
        let line_comment = match style {
            Style::CLike => starts(i, "//"),
            Style::Hash => c == '#',
            _ => false,
        };
        if line_comment {
            while i < chars.len() && chars[i] != '\n' {
                i += 1;
            }
            continue;
        }
        let block = match style {
            Style::CLike if starts(i, "/*") => Some(("/*", "*/")),
            Style::Html if starts(i, "<!--") => Some(("<!--", "-->")),
            _ => None,
        };
        if let Some((open, close)) = block {
            i += open.len();
            while i < chars.len() && !starts(i, close) {
                if chars[i] == '\n' {
                    out.push('\n');
                }
                i += 1;
            }
            i = (i + close.len()).min(chars.len());
            continue;
        }
        match (style, c) {
            (Style::CLike, '"' | '`') | (Style::Hash, '"' | '\'') => quote = Some(c),
            (Style::CLike, '\'') => {
                // A character literal, such as '/' or '\n'. A ' that starts
                // none, such as a Rust lifetime, is only itself.
                if let Some(length) = char_literal(&chars[i..]) {
                    out.extend(&chars[i..i + length]);
                    i += length;
                    continue;
                }
            }
            _ => {}
        }
        out.push(c);
        i += 1;
    }
    out.split('\n')
        .map(str::trim_end)
        .collect::<Vec<_>>()
        .join("\n")
}

/// The length of the character literal `chars` starts with, such as 3 for
/// `'a'` or 4 for `'\n'`, or `None` if it doesn't start one.
fn char_literal(chars: &[char]) -> Option<usize> {
    match chars.get(1)? {
        '\\' => (3..chars.len().min(12))
            .find(|&k| chars[k] == '\'')
            .map(|k| k + 1),
        '\n' | '\'' => None,
        _ => (chars.get(2) == Some(&'\'')).then_some(3),
    }
}

/// A diff, as `git show` prints it, with comments stripped from each
/// file's changed and context lines in that file's style. Every other line,
/// such as a commit's subject or a hunk header, is kept as it is.
fn strip_diff(diff: &str) -> String {
    let mut out: Vec<String> = Vec::new();
    let mut hunk: Vec<(char, String)> = Vec::new();
    let mut style = Style::None;
    let mut in_hunk = false;
    let flush = |hunk: &mut Vec<(char, String)>, out: &mut Vec<String>, style: Style| {
        if hunk.is_empty() {
            return;
        }
        let text: Vec<&str> = hunk.iter().map(|(_, line)| line.as_str()).collect();
        let stripped = strip_comments(&text.join("\n"), style);
        for ((marker, _), line) in hunk.iter().zip(stripped.split('\n')) {
            out.push(format!("{marker}{line}"));
        }
        hunk.clear();
    };
    for line in diff.split('\n') {
        let marker = line.chars().next();
        if in_hunk && matches!(marker, Some('+' | '-' | ' ')) {
            let marker = marker.unwrap_or(' ');
            hunk.push((marker, line[marker.len_utf8()..].to_string()));
            continue;
        }
        flush(&mut hunk, &mut out, style);
        if let Some(rest) = line.strip_prefix("diff --git ") {
            style = style_for(rest.rsplit(" b/").next().unwrap_or(rest));
        }
        in_hunk = line.starts_with("@@");
        out.push(line.to_string());
    }
    flush(&mut hunk, &mut out, style);
    out.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn c_like_comments_go_and_strings_and_line_breaks_stay() {
        let code = "let url = \"http://x\"; // a comment\n/* one\ntwo */ let c = '/';\nfn f<'a>(x: &'a str) {} // it's\n";
        assert_eq!(
            strip_comments(code, Style::CLike),
            "let url = \"http://x\";\n\n let c = '/';\nfn f<'a>(x: &'a str) {}\n"
        );
    }

    #[test]
    fn hash_comments_go_but_not_inside_strings() {
        let code = "x = '#not' # gone\n# all gone\ny = 1\n";
        assert_eq!(strip_comments(code, Style::Hash), "x = '#not'\n\ny = 1\n");
    }

    #[test]
    fn a_diff_is_stripped_in_each_files_style_and_its_headers_kept() {
        let diff = "commit abc\nfeat(XX-1): // not a comment\ndiff --git a/src/a.rs b/src/a.rs\n--- a/src/a.rs\n+++ b/src/a.rs\n@@ -1 +1 @@\n-old(); // gone\n+new(); // gone too\n";
        assert_eq!(
            strip_diff(diff),
            "commit abc\nfeat(XX-1): // not a comment\ndiff --git a/src/a.rs b/src/a.rs\n--- a/src/a.rs\n+++ b/src/a.rs\n@@ -1 +1 @@\n-old();\n+new();\n"
        );
    }
}

#[cfg(test)]
mod tr45_tests {
    use super::*;

    #[test]
    fn a_no_newline_marker_stays_in_its_hunk_and_stripping_goes_on() {
        let diff = "diff --git a/a.rs b/a.rs\n--- a/a.rs\n+++ b/a.rs\n@@ -1,2 +1,3 @@\n-fn b() {  }\n\\ No newline at end of file\n+fn b() { /* trust me */ }\n+fn c() {} // fine\n\\ No newline at end of file\n";
        assert_eq!(
            strip_diff(diff),
            "diff --git a/a.rs b/a.rs\n--- a/a.rs\n+++ b/a.rs\n@@ -1,2 +1,3 @@\n-fn b() {  }\n\\ No newline at end of file\n+fn b() {  }\n+fn c() {}\n\\ No newline at end of file\n"
        );
    }
}
