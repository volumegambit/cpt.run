use std::time::Duration;

pub(crate) const APP_VERSION: &str = env!("CARGO_PKG_VERSION");
pub(crate) const TICK_RATE: Duration = Duration::from_millis(200);

pub(crate) const COMMAND_HELP: &str = concat!(
    "Commands: /help, /add <text>, /edit <id> [text], /next [id], /done [id], ",
    "/delete [id], /filter (clear), /refresh, /view|/tab <name>, /quit"
);

pub(crate) const STATUS_ENTER_ADD: &str =
    "Enter a task description, tokens allowed (Esc to cancel)";
pub(crate) const STATUS_COMMAND_PALETTE: &str =
    "Type a /command • Up/Down: navigate • Tab/Right: complete • Enter: run • Esc: cancel";
pub(crate) const STATUS_FILTER_PICKER: &str =
    "Filter picker — ←/→ column • Tab/Shift+Tab cycle • ↑/↓ move • Space toggle • C clears all • Enter apply • Esc cancel";
pub(crate) const STATUS_REFRESHED: &str = "Refreshed tasks";
pub(crate) const STATUS_PROJECT_MOVE: &str = "Select a task view to move items";
pub(crate) const STATUS_PROJECT_DONE: &str = "Select a task view to mark items done";
pub(crate) const STATUS_PROJECT_DELETE: &str = "Select a task view to delete items";
pub(crate) const STATUS_PROJECT_EDIT: &str = "Select a task view to edit items";
pub(crate) const STATUS_ENTER_EDIT: &str =
    "Edit task — adjust tokens, tokens apply immediately • Enter to save • Esc to cancel";
pub(crate) const STATUS_PROJECT_INBOX: &str = "Select a task view to send items back to Inbox";
pub(crate) const STATUS_PROJECT_SOMEDAY: &str = "Select a task view to move items into Someday";
pub(crate) const STATUS_VIEW_DETAILS: &str = "Viewing task details • Enter/Esc to close";
pub(crate) const STATUS_HELP: &str = "Keyboard reference — Enter/Esc to close";
pub(crate) const STATUS_CONFIRM_DELETE: &str =
    "Confirm deletion — arrows choose, Enter confirms, Esc cancels";
