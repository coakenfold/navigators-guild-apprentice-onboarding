//! TR-40: `tracker submit --help` names every trigger.

mod common;

use common::Env;

#[test]
fn submit_help_names_every_trigger() {
    let env = Env::without_store();
    let output = env.cmd().args(["submit", "--help"]).assert().success();
    let help = String::from_utf8(output.get_output().stdout.clone()).unwrap();
    for trigger in [
        "T1", "T2", "T3", "T4", "T5", "T6", "T7", "T8", "T9", "T13", "T14",
    ] {
        let named = help
            .split(|c: char| !c.is_ascii_alphanumeric())
            .any(|word| word == trigger);
        assert!(named, "submit --help doesn't name {trigger}:\n{help}");
    }
}
