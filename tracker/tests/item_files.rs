//! Item files are read back only as the item their name says, so a file
//! copied or edited by hand can't pass for another item (§4.4).

mod common;

use std::fs;

use common::Env;
use tracker::store::Store;

fn store(env: &Env) -> Store {
    Store {
        home: env.home.path().to_path_buf(),
    }
}

#[test]
fn a_file_whose_name_and_id_disagree_is_refused() {
    let env = Env::new();
    env.add_project("XX", "xx");
    env.create("XX", "a");
    // A copy made by hand: XX-9.json still says it's XX-1.
    fs::copy(env.item_path("XX-1"), env.item_path("XX-9")).unwrap();

    for args in [&["list"][..], &["project", "list"]] {
        let message = env.error(args, 3);
        assert!(message.contains("XX-9.json"), "{args:?}: {message}");
        assert!(message.contains("XX-1"), "{args:?}: {message}");
    }
    let error = store(&env).load_item("XX-9").unwrap_err();
    assert_eq!(error.code, 3);
}

#[test]
fn a_file_whose_project_and_id_disagree_is_refused() {
    let env = Env::new();
    env.add_project("XX", "xx");
    env.add_project("YY", "yy");
    env.create("XX", "a");
    let path = env.item_path("XX-1");
    let text = fs::read_to_string(&path).unwrap();
    fs::write(
        &path,
        text.replace("\"project\": \"XX\"", "\"project\": \"YY\""),
    )
    .unwrap();

    let message = env.error(&["list"], 3);
    assert!(message.contains("XX-1.json"), "{message}");
    assert!(message.contains("\"project\""), "{message}");
}

#[test]
fn a_json_file_not_named_after_an_id_is_refused() {
    let env = Env::new();
    env.add_project("XX", "xx");
    let text = {
        env.create("XX", "a");
        fs::read_to_string(env.item_path("XX-1")).unwrap()
    };
    fs::write(env.home.path().join("items/notes.json"), text).unwrap();

    let message = env.error(&["list"], 3);
    assert!(message.contains("notes.json"), "{message}");
}

#[test]
fn load_item_reads_one_item_and_says_when_there_is_none() {
    let env = Env::new();
    env.add_project("XX", "xx");
    env.create("XX", "a");
    let store = store(&env);

    assert_eq!(store.load_item("XX-1").unwrap().title, "a");
    let error = store.load_item("XX-2").unwrap_err();
    assert_eq!(error.code, 1);
    assert!(error.message.contains("no item XX-2"), "{}", error.message);
}
