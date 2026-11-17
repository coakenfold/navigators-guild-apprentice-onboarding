//! Whether a command was typed by me at a terminal (§2.7).

use std::io::IsTerminal;

/// Whether stdin is a terminal. An agent's shell calls never are, so this
/// is how tracker tells my actions from an agent's. History's `interactive`
/// flag uses it, and so does slice 4's check for human-only commands, so the
/// two always agree on what counts as me.
///
/// It's a convention, not security: a command run under `script` gets a
/// terminal too (§7). The digest lists my actions as well as the agent's
/// for that reason (§2.6).
pub fn is_interactive() -> bool {
    std::io::stdin().is_terminal()
}
