//! Whether a command was typed by me at a terminal, and the confirmation
//! that the human-only commands ask for (§2.7).

use std::io::{BufRead, IsTerminal, Write};

use crate::error::{Error, Result};
use crate::text;

/// Whether stdin is a terminal. An agent's shell calls never are, so this
/// is how tracker tells my actions from an agent's. History's `interactive`
/// flag uses it, and so does `confirm`, so the two always agree on what
/// counts as me.
///
/// It's a convention, not security: a command run under `script` gets a
/// terminal too (§7). The digest lists my actions as well as the agent's
/// for that reason (§2.6).
pub fn is_interactive() -> bool {
    std::io::stdin().is_terminal()
}

/// Step 1 of `confirm` on its own: refuses, with exit 1, unless stdin is a
/// terminal. For a human-only command that may have nothing to confirm,
/// such as `criterion accept` when `written` already matches, so that it
/// refuses an agent either way.
pub fn require_interactive(command: &str, id: &str, otherwise: &str) -> Result<()> {
    if is_interactive() {
        return Ok(());
    }
    let mut message = format!(
        "`{command}` needs a human at an interactive terminal, who types {id} to confirm it, \
         and stdin here isn't a terminal. Nothing was changed."
    );
    if !otherwise.is_empty() {
        message.push(' ');
        message.push_str(otherwise);
    }
    Err(Error::input(message))
}

/// The confirmation every human-only command asks for (§2.7): `delete`,
/// `criterion accept`, `approve` and `reject` now, and later `digest ack`.
///
/// 1. Refuses, with exit 1, unless stdin is a terminal (`is_interactive`).
///    `command` is how the command was run, such as `tracker delete XX-1`,
///    and `otherwise` says what an agent can do instead, or is empty.
/// 2. Prints `warning`, which says what will be lost, then asks for `id`:
///    the item ID, or `ack` for `digest ack`, which acts on a project.
/// 3. Reads one line. Anything but `id` cancels, with exit 1, and so does
///    an empty line or the end of input. The ID's key, or `ack`, may be
///    typed in either case, as on the command line.
///
/// The warning and question go to stderr, so a `--json` caller's stdout
/// still holds one JSON object. Call this before taking the store's lock:
/// the lock would stop every other command while I read and decide. Then
/// take the lock with `Store::lock_unchanged`, which refuses if the item
/// changed from what the warning described (§4.4).
pub fn confirm(command: &str, id: &str, warning: &str, otherwise: &str) -> Result<()> {
    require_interactive(command, id, otherwise)?;

    // Escaped like every other output, since the warning can hold a title
    // edited by hand.
    let mut stderr = std::io::stderr().lock();
    let asked = write!(
        stderr,
        "{}Type {id} to confirm, or anything else to cancel: ",
        text::for_terminal(warning, true)
    )
    .and_then(|()| stderr.flush());
    if let Err(e) = asked {
        return Err(Error::store(format!(
            "Can't ask for {id} to be typed: {e}. Nothing was changed."
        )));
    }
    drop(stderr);

    let mut answer = String::new();
    let read = std::io::stdin().lock().read_line(&mut answer);
    let answer = answer.trim();
    match read {
        Ok(_) if answer.eq_ignore_ascii_case(id) => Ok(()),
        Ok(0) => Err(Error::input(format!(
            "Cancelled: the input ended before {id} was typed. Nothing was changed."
        ))),
        Ok(_) if answer.is_empty() => Err(Error::input(format!(
            "Cancelled: nothing was typed, and {id} is needed to confirm. Nothing was changed."
        ))),
        // Every error message is escaped when it's printed, so what was
        // typed can't change the display.
        Ok(_) => Err(Error::input(format!(
            "Cancelled: \"{answer}\" isn't {id}. Nothing was changed."
        ))),
        Err(e) => Err(Error::input(format!(
            "Cancelled: can't read what was typed: {e}. Nothing was changed."
        ))),
    }
}
