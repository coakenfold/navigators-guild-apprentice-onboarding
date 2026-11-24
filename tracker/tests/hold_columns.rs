//! TR-39: the hold messages in `submit`, `gates`, `review`, `show` and
//! `approve`'s question start in one column when the triggers differ in
//! width.

mod common;

use common::Env;

const PROMPT: &str = "Type XX-1 to confirm";

/// XX-1, held under T9 (the `needs-review` label) and T13 (a check that
/// already passed at start), so the triggers are two and three characters
/// wide. Returns the env and the submit's text.
fn held_by_t9_and_t13() -> (Env, String) {
    let env = Env::new();
    env.add_project("XX", "xx");
    env.create_bare("XX", "wide");
    env.json(&["criterion", "add", "XX-1", "it works", "--check", "true"]);
    env.json(&["edit", "XX-1", "--label", "needs-review"]);
    env.json(&["start", "XX-1"]);
    let text = text(&env, &["submit", "XX-1"]);
    (env, text)
}

fn text(env: &Env, args: &[&str]) -> String {
    let output = env.cmd().args(args).output().unwrap();
    String::from_utf8(output.stdout).unwrap()
}

/// Asserts that `output` has hold lines for T9 and T13, and that their
/// messages start in the same column.
fn assert_one_column(output: &str) {
    let column = |trigger: &str| -> usize {
        let prefix = format!("  {trigger} ");
        let line = output
            .lines()
            .map(|l| l.trim_end_matches('\r'))
            .find(|l| l.starts_with(&prefix))
            .unwrap_or_else(|| panic!("no {trigger} line in:\n{output}"));
        let after = &line[prefix.len()..];
        prefix.len() + (after.len() - after.trim_start().len())
    };
    assert_eq!(column("T9"), column("T13"), "{output}");
}

#[test]
fn submit_lines_up_holds_of_different_widths() {
    let (_env, submitted) = held_by_t9_and_t13();
    assert_one_column(&submitted);
}

#[test]
fn review_lines_up_holds_of_different_widths() {
    let (env, _) = held_by_t9_and_t13();
    assert_one_column(&text(&env, &["review"]));
}

#[test]
fn show_lines_up_holds_of_different_widths() {
    let (env, _) = held_by_t9_and_t13();
    assert_one_column(&text(&env, &["show", "XX-1"]));
}

#[test]
fn gates_lines_up_holds_of_different_widths() {
    let (env, _) = held_by_t9_and_t13();
    assert_one_column(&text(&env, &["gates", "XX-1"]));
}

#[test]
fn approves_question_lines_up_holds_of_different_widths() {
    let (env, _) = held_by_t9_and_t13();
    let Some((output, _)) = env.in_terminal_typing(&["approve", "XX-1"], PROMPT, "no", || {})
    else {
        return;
    };
    assert_one_column(&output);
}
