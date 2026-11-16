//! Only an item ID can become an item file's path (§4.4), so an ID typed by
//! a person or an agent, such as `../config`, can't name another file.

use std::path::PathBuf;

use tracker::store::Store;

fn store() -> Store {
    Store {
        home: PathBuf::from("/store"),
    }
}

#[test]
fn an_item_id_maps_to_its_file_in_items() {
    for id in ["BM-12", "TR-1", "A1B2-100", "ABCDEFGHIJ-9"] {
        let path = store().item_path(id).unwrap();
        assert_eq!(path, PathBuf::from(format!("/store/items/{id}.json")));
    }
}

#[test]
fn anything_else_is_refused() {
    for id in [
        "../config",
        "../../etc/passwd",
        "BM-12/../../x",
        "bm-12",
        "BM-0",
        "BM-012",
        "BM-",
        "BM",
        "-12",
        "BM-1-2",
        "BM-1.json",
        "BM-+1",
        "ABCDEFGHIJK-1",
        "BM-99999999999999999999999",
        "",
    ] {
        let error = store().item_path(id).unwrap_err();
        assert_eq!(error.code, 1, "{id:?}");
        assert!(error.message.contains("isn't an item ID"), "{id:?}");
    }
}
