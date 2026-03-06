//! Terminal window title management via crossterm's SetTitle command.
//! Works with any xterm-compatible terminal: iTerm2, Kitty, Terminal.app,
//! gnome-terminal, konsole, WezTerm, Ghostty, etc.

use crossterm::{execute, terminal::SetTitle};
use std::path::Path;

/// Update the terminal window title to show the current directory.
///
/// Uses crossterm's [`SetTitle`] command, which emits the standard OSC 0
/// escape sequence (`ESC ] 0 ; title BEL`) supported by all xterm-compatible
/// terminals.
pub fn set_window_title(path: &Path) {
    let title = format!("fang — {}", path.display());
    let _ = execute!(std::io::stdout(), SetTitle(&title));
}

/// Reset the window title to a plain "fang" on exit.
#[allow(dead_code)]
pub fn reset_window_title() {
    let _ = execute!(std::io::stdout(), SetTitle("fang"));
}
