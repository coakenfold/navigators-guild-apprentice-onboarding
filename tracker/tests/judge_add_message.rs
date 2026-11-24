//! TR-52: what `criterion add --judge` says about when its question is asked.

mod common;

use common::Env;

#[test]
fn tr52_c1_add_judge_says_the_question_is_asked_at_start_and_at_submit_once_other_gates_pass() {
    let env = Env::new();
    let root = env.add_project("XX", "xx");
    std::fs::create_dir_all(root.join("src")).unwrap();
    std::fs::write(root.join("src/lib.rs"), "fn run() {}\n").unwrap();
    env.json(&["create", "a", "--project", "XX"]);
    let path = env.dir("judge").join("judge.json");
    std::fs::write(
        &path,
        r#"{"text":"The code does the work","inputs":["src/lib.rs"],"ask":[{"id":"works","ask":"Does this code do the work?"}]}"#,
    )
    .unwrap();
    let output = env
        .cmd()
        .current_dir(env.outside())
        .args([
            "criterion",
            "add",
            "XX-1",
            "--judge",
            path.to_str().unwrap(),
        ])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let text = String::from_utf8(output).unwrap();
    assert!(
        text.contains("asked at start, and at submit once every other gate passes"),
        "{text}"
    );
}
