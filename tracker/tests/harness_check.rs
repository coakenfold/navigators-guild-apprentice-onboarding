//! TR-41: the harness check that reads its parent's arguments.

mod common;

use common::BEFORE_THE_WORK_FAILS;

#[test]
fn before_the_work_fails_reads_the_parents_arguments_with_ps_ww() {
    // Without `-ww`, `ps` cuts a long command line to the terminal's width.
    assert!(
        BEFORE_THE_WORK_FAILS.contains("ps -ww -o args="),
        "{BEFORE_THE_WORK_FAILS}"
    );
}
