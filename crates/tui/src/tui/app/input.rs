use anyhow::Result;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::tui::constants::{
    STATUS_COMMAND_PALETTE, STATUS_ENTER_ADD, STATUS_PROJECT_DELETE, STATUS_PROJECT_DONE,
    STATUS_PROJECT_INBOX, STATUS_PROJECT_MOVE, STATUS_PROJECT_SOMEDAY, STATUS_REFRESHED,
};

use super::{App, ConfirmChoice, InputMode};

#[derive(Debug, Clone, Copy)]
pub(crate) struct CommandTrigger {
    initial_input: &'static str,
    status: &'static str,
}

impl CommandTrigger {
    const fn new(initial_input: &'static str, status: &'static str) -> Self {
        Self {
            initial_input,
            status,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub(crate) enum NormalAction {
    Quit,
    EnterAdd,
    EnterEdit,
    ShowDetails,
    ShowHelp,
    Refresh,
    OpenFilter,
    EnterCommand(CommandTrigger),
    MarkNext,
    MarkSomeday,
    MarkInbox,
    MarkDone,
    Delete,
    SelectNext,
    SelectPrev,
    PrevTab,
    NextTab,
    SelectFirst,
    SelectLast,
}

impl NormalAction {
    fn from_event(key: &KeyEvent) -> Option<Self> {
        if matches!(key.code, KeyCode::Char('c')) && key.modifiers.contains(KeyModifiers::CONTROL) {
            return Some(Self::Quit);
        }

        match key.code {
            KeyCode::Char('q') => Some(Self::Quit),
            KeyCode::Char('a') => Some(Self::EnterAdd),
            KeyCode::Char('e') => Some(Self::EnterEdit),
            KeyCode::Char('i') => Some(Self::MarkInbox),
            KeyCode::Char('s') => Some(Self::MarkSomeday),
            KeyCode::Char('r') => Some(Self::Refresh),
            KeyCode::Char('f') => Some(Self::OpenFilter),
            KeyCode::Char('h') => Some(Self::ShowHelp),
            KeyCode::Char('/') => Some(Self::EnterCommand(CommandTrigger::new(
                "/",
                STATUS_COMMAND_PALETTE,
            ))),
            KeyCode::Char('n') => Some(Self::MarkNext),
            KeyCode::Char('d') => Some(Self::MarkDone),
            KeyCode::Char('x') | KeyCode::Delete => Some(Self::Delete),
            KeyCode::Char('j') | KeyCode::Down => Some(Self::SelectNext),
            KeyCode::Char('k') | KeyCode::Up => Some(Self::SelectPrev),
            KeyCode::Left | KeyCode::BackTab => Some(Self::PrevTab),
            KeyCode::Char('l') | KeyCode::Right | KeyCode::Tab => Some(Self::NextTab),
            KeyCode::Enter => Some(Self::ShowDetails),
            KeyCode::Home => Some(Self::SelectFirst),
            KeyCode::End => Some(Self::SelectLast),
            _ => None,
        }
    }
}

impl App {
    pub(crate) fn on_key(&mut self, key: KeyEvent) -> Result<()> {
        match self.input_mode {
            InputMode::Normal => self.handle_normal_mode(key),
            InputMode::Add => self.handle_add_mode(key),
            InputMode::Command => self.handle_command_mode(key),
            InputMode::Filter => self.handle_filter_mode(key),
            InputMode::Edit => self.handle_edit_mode(key),
            InputMode::Inspect => self.handle_inspect_mode(key),
            InputMode::Help => self.handle_help_mode(key),
            InputMode::ConfirmDelete => self.handle_confirm_delete_mode(key),
        }
    }

    fn handle_normal_mode(&mut self, key: KeyEvent) -> Result<()> {
        if let Some(action) = NormalAction::from_event(&key) {
            self.execute_normal_action(action)?;
        }
        Ok(())
    }

    fn handle_filter_mode(&mut self, key: KeyEvent) -> Result<()> {
        if self.filter_overlay.is_none() {
            self.input_mode = InputMode::Normal;
            return Ok(());
        }

        let mut apply = false;
        let mut cancel = false;

        match key.code {
            KeyCode::Esc => cancel = true,
            KeyCode::Enter => apply = true,
            KeyCode::Left => {
                if let Some(overlay) = self.filter_overlay.as_mut() {
                    overlay.prev_column();
                }
            }
            KeyCode::Right | KeyCode::Tab => {
                if let Some(overlay) = self.filter_overlay.as_mut() {
                    overlay.next_column();
                }
            }
            KeyCode::BackTab => {
                if let Some(overlay) = self.filter_overlay.as_mut() {
                    overlay.prev_column();
                }
            }
            KeyCode::Up => {
                if let Some(overlay) = self.filter_overlay.as_mut() {
                    overlay.prev_row();
                }
            }
            KeyCode::Down => {
                if let Some(overlay) = self.filter_overlay.as_mut() {
                    overlay.next_row();
                }
            }
            KeyCode::Char(' ') => {
                if let Some(overlay) = self.filter_overlay.as_mut() {
                    overlay.toggle_current();
                }
            }
            KeyCode::Char('c') | KeyCode::Char('C') => {
                let mut cleared = false;
                if let Some(overlay) = self.filter_overlay.as_mut() {
                    overlay.clear_all();
                    cleared = true;
                }
                if cleared {
                    self.set_status_info("Cleared filter selections — press Enter to apply");
                }
            }
            _ => {}
        }

        if apply {
            if let Some(overlay) = self.filter_overlay.take() {
                self.active_filters = overlay.commit();
                self.input_mode = InputMode::Normal;
                self.refresh()?;
                let status = if let Some(summary) = self.active_filters.summary() {
                    format!("Applied filters: {summary}")
                } else {
                    String::from("Cleared filters")
                };
                self.set_status_info(status);
            }
        } else if cancel {
            if let Some(overlay) = self.filter_overlay.take() {
                self.active_filters = overlay.cancel();
                self.input_mode = InputMode::Normal;
                let status = if let Some(summary) = self.active_filters.summary() {
                    format!("Filters unchanged: {summary}")
                } else {
                    String::from("Filters unchanged")
                };
                self.set_status_info(status);
            }
        }

        Ok(())
    }

    fn execute_normal_action(&mut self, action: NormalAction) -> Result<()> {
        match action {
            NormalAction::Quit => {
                self.should_quit = true;
            }
            NormalAction::EnterAdd => {
                self.input_mode = InputMode::Add;
                self.input.clear();
                self.set_status_info(STATUS_ENTER_ADD);
            }
            NormalAction::EnterEdit => {
                self.start_edit_current()?;
            }
            NormalAction::ShowDetails => {
                self.show_selected_details()?;
            }
            NormalAction::ShowHelp => {
                self.show_help_overlay();
            }
            NormalAction::MarkInbox => {
                if self.ensure_task_view(STATUS_PROJECT_INBOX) {
                    self.mark_inbox()?;
                }
            }
            NormalAction::MarkSomeday => {
                if self.ensure_task_view(STATUS_PROJECT_SOMEDAY) {
                    self.mark_someday()?;
                }
            }
            NormalAction::Refresh => {
                self.refresh()?;
                self.set_status_info(STATUS_REFRESHED);
            }
            NormalAction::OpenFilter => {
                self.open_filter_overlay()?;
            }
            NormalAction::EnterCommand(trigger) => {
                self.input_mode = InputMode::Command;
                self.input.set(trigger.initial_input);
                self.update_command_suggestions();
                self.set_status_info(trigger.status);
            }
            NormalAction::MarkNext => {
                if self.ensure_task_view(STATUS_PROJECT_MOVE) {
                    self.mark_next()?;
                }
            }
            NormalAction::MarkDone => {
                if self.ensure_task_view(STATUS_PROJECT_DONE) {
                    self.mark_done()?;
                }
            }
            NormalAction::Delete => {
                if self.ensure_task_view(STATUS_PROJECT_DELETE) {
                    self.prompt_delete();
                }
            }
            NormalAction::SelectNext => self.select_next(),
            NormalAction::SelectPrev => self.select_prev(),
            NormalAction::PrevTab => {
                self.prev_tab()?;
            }
            NormalAction::NextTab => {
                self.next_tab()?;
            }
            NormalAction::SelectFirst => {
                if !self.tasks.is_empty() {
                    self.selected = 0;
                    self.table_state.select(Some(self.selected));
                }
            }
            NormalAction::SelectLast => {
                if !self.tasks.is_empty() {
                    self.selected = self.tasks.len() - 1;
                    self.table_state.select(Some(self.selected));
                }
            }
        }
        Ok(())
    }

    fn handle_add_mode(&mut self, key: KeyEvent) -> Result<()> {
        match key.code {
            KeyCode::Enter => {
                if key.modifiers.contains(KeyModifiers::SHIFT) {
                    self.input.insert_newline();
                    Ok(())
                } else {
                    self.add_task()
                }
            }
            KeyCode::Esc => {
                self.input_mode = InputMode::Normal;
                self.status = None;
                Ok(())
            }
            KeyCode::Backspace => {
                self.input.backspace();
                Ok(())
            }
            KeyCode::Delete => {
                self.input.delete_char();
                Ok(())
            }
            KeyCode::Char(c) => {
                self.input.insert_char(c);
                Ok(())
            }
            KeyCode::Tab => {
                self.input.insert_tab();
                Ok(())
            }
            KeyCode::Left => {
                self.input.move_left();
                Ok(())
            }
            KeyCode::Right => {
                self.input.move_right();
                Ok(())
            }
            KeyCode::Up => {
                self.input.move_up();
                Ok(())
            }
            KeyCode::Down => {
                self.input.move_down();
                Ok(())
            }
            KeyCode::Home => {
                self.input.move_home();
                Ok(())
            }
            KeyCode::End => {
                self.input.move_end();
                Ok(())
            }
            _ => Ok(()),
        }
    }

    fn handle_edit_mode(&mut self, key: KeyEvent) -> Result<()> {
        match key.code {
            KeyCode::Enter => {
                if key.modifiers.contains(KeyModifiers::SHIFT) {
                    self.input.insert_newline();
                    Ok(())
                } else {
                    self.apply_edit()
                }
            }
            KeyCode::Esc => {
                self.cancel_edit();
                Ok(())
            }
            KeyCode::Backspace => {
                self.input.backspace();
                Ok(())
            }
            KeyCode::Delete => {
                self.input.delete_char();
                Ok(())
            }
            KeyCode::Char(c) => {
                self.input.insert_char(c);
                Ok(())
            }
            KeyCode::Tab => {
                self.input.insert_tab();
                Ok(())
            }
            KeyCode::Left => {
                self.input.move_left();
                Ok(())
            }
            KeyCode::Right => {
                self.input.move_right();
                Ok(())
            }
            KeyCode::Up => {
                self.input.move_up();
                Ok(())
            }
            KeyCode::Down => {
                self.input.move_down();
                Ok(())
            }
            KeyCode::Home => {
                self.input.move_home();
                Ok(())
            }
            KeyCode::End => {
                self.input.move_end();
                Ok(())
            }
            _ => Ok(()),
        }
    }

    fn handle_inspect_mode(&mut self, key: KeyEvent) -> Result<()> {
        match key.code {
            KeyCode::Esc | KeyCode::Enter => {
                self.inspect_task = None;
                self.input_mode = InputMode::Normal;
                self.status = None;
                Ok(())
            }
            _ => Ok(()),
        }
    }

    fn handle_help_mode(&mut self, key: KeyEvent) -> Result<()> {
        match key.code {
            KeyCode::Esc | KeyCode::Enter => {
                self.input_mode = InputMode::Normal;
                self.status = None;
                Ok(())
            }
            _ => Ok(()),
        }
    }

    fn handle_confirm_delete_mode(&mut self, key: KeyEvent) -> Result<()> {
        match key.code {
            KeyCode::Esc => {
                self.input_mode = InputMode::Normal;
                self.set_status_info("Deletion cancelled");
                Ok(())
            }
            KeyCode::Left | KeyCode::Right | KeyCode::Char(' ') => {
                self.confirm_choice = self.confirm_choice.toggle();
                Ok(())
            }
            KeyCode::Enter => {
                if self.confirm_choice == ConfirmChoice::Yes {
                    self.perform_delete()?;
                } else {
                    self.set_status_info("Deletion cancelled");
                }
                self.input_mode = InputMode::Normal;
                Ok(())
            }
            _ => Ok(()),
        }
    }

    fn handle_command_mode(&mut self, key: KeyEvent) -> Result<()> {
        match key.code {
            KeyCode::Enter => {
                if let Some(s) = self.suggestions.get(self.suggestion_index) {
                    if s.fill.ends_with(' ') {
                        self.input.set(s.fill.clone());
                        self.update_command_suggestions();
                        Ok(())
                    } else {
                        self.input.set(s.fill.clone());
                        self.run_command()
                    }
                } else {
                    self.run_command()
                }
            }
            KeyCode::Esc => {
                self.input_mode = InputMode::Normal;
                self.status = None;
                Ok(())
            }
            KeyCode::Backspace => {
                self.input.backspace();
                self.update_command_suggestions();
                Ok(())
            }
            KeyCode::Char(c) => {
                self.input.insert_char(c);
                self.update_command_suggestions();
                Ok(())
            }
            KeyCode::Delete => {
                self.input.delete_char();
                self.update_command_suggestions();
                Ok(())
            }
            KeyCode::Tab | KeyCode::Right => {
                self.accept_suggestion();
                Ok(())
            }
            KeyCode::Up => {
                if !self.suggestions.is_empty() {
                    if self.suggestion_index == 0 {
                        self.suggestion_index = self.suggestions.len() - 1;
                    } else {
                        self.suggestion_index -= 1;
                    }
                }
                Ok(())
            }
            KeyCode::Down => {
                if !self.suggestions.is_empty() {
                    self.suggestion_index = (self.suggestion_index + 1) % self.suggestions.len();
                }
                Ok(())
            }
            _ => Ok(()),
        }
    }
}
