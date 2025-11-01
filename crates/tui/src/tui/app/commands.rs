use anyhow::Result;

use crate::tui::constants::COMMAND_HELP;

use super::App;

#[derive(Debug, Clone)]
pub(crate) struct Suggestion {
    pub(crate) fill: String,
    pub(crate) label: String,
}

impl App {
    pub(crate) fn run_command(&mut self) -> Result<()> {
        let raw = self.input.as_str().trim();
        if !raw.starts_with('/') {
            self.set_status_error("Commands must start with '/'");
            return Ok(());
        }
        let mut parts = raw[1..].split_whitespace();
        let cmd = match parts.next() {
            Some(c) => c.to_ascii_lowercase(),
            None => {
                self.set_status_error("Enter a command after '/'");
                return Ok(());
            }
        };

        match cmd.as_str() {
            "help" | "h" => {
                self.set_status_info(COMMAND_HELP);
            }
            "add" => {
                let rest: Vec<String> = parts.map(|s| s.to_string()).collect();
                if rest.is_empty() {
                    self.set_status_error("Usage: /add <task description>");
                } else {
                    self.input.set(rest.join(" "));
                    self.add_task()?;
                    return Ok(());
                }
            }
            "next" => {
                if let Some(id) = parts.next() {
                    let results = self.database.mark_next(&[id.to_string()])?;
                    if results.iter().any(|r| r.changed) {
                        self.set_status_info("Moved task to next actions");
                    } else {
                        self.set_status_info("Task not found or already in next actions");
                    }
                    self.refresh()?;
                } else {
                    self.mark_next()?;
                }
            }
            "done" => {
                if let Some(id) = parts.next() {
                    let results = self.database.mark_done(&[id.to_string()])?;
                    if results.iter().any(|r| r.changed) {
                        self.set_status_info("Marked task as done");
                    } else {
                        self.set_status_info("Task not found or already done");
                    }
                    self.refresh()?;
                } else {
                    self.mark_done()?;
                }
            }
            "delete" | "del" | "rm" => {
                if let Some(id) = parts.next() {
                    let results = self.database.delete_tasks(&[id.to_string()])?;
                    if results.iter().any(|r| r.deleted) {
                        self.set_status_info("Deleted task 🗑️");
                    } else {
                        self.set_status_info("Task not found");
                    }
                    self.refresh()?;
                } else {
                    self.prompt_delete();
                }
            }
            "edit" => {
                let rest: Vec<String> = parts.map(|s| s.to_string()).collect();
                if rest.is_empty() {
                    self.start_edit_current()?;
                    self.finish_command();
                    return Ok(());
                }

                let id = rest[0].clone();
                if rest.len() == 1 {
                    self.start_edit_by_id(id)?;
                    self.finish_command();
                    return Ok(());
                }

                let text = rest[1..].join(" ");
                self.edit_task_with_text(id.clone(), &text)?;
                self.finish_command();
                return Ok(());
            }
            "filter" => {
                let rest: Vec<String> = parts.map(|s| s.to_string()).collect();
                if rest.is_empty() {
                    self.input.clear();
                    self.open_filter_overlay()?;
                    return Ok(());
                }

                if rest.first().is_some_and(|val| {
                    val.eq_ignore_ascii_case("clear") || val.eq_ignore_ascii_case("off")
                }) {
                    self.active_filters = Default::default();
                    self.refresh()?;
                    self.set_status_info("Cleared filters");
                } else {
                    self.set_status_error(
                        "Filter picker no longer accepts text. Press 'f' to open it.",
                    );
                }
            }
            "refresh" | "r" => {
                self.refresh()?;
                self.set_status_info("Refreshed tasks");
            }
            "quit" | "q" | "exit" => {
                self.should_quit = true;
            }
            "view" | "tab" => {
                if let Some(name) = parts.next() {
                    let name = name.to_ascii_lowercase();
                    let idx = match name.as_str() {
                        "all" => 0,
                        "inbox" => 1,
                        "next" => 2,
                        "waiting" => 3,
                        "scheduled" => 4,
                        "someday" => 5,
                        "projects" => 6,
                        "done" => 7,
                        _ => {
                            self.set_status_error("Unknown view; try: all/inbox/next/waiting/scheduled/someday/projects/done");
                            self.finish_command();
                            return Ok(());
                        }
                    };
                    self.tab_index = idx;
                    self.refresh()?;
                } else {
                    self.set_status_error("Usage: /view <tab>");
                }
            }
            unknown => {
                self.set_status_error(format!("Unknown command: {} (try /help)", unknown));
            }
        }

        self.finish_command();
        Ok(())
    }

    pub(crate) fn finish_command(&mut self) {
        self.input.clear();
        self.input_mode = super::InputMode::Normal;
    }

    pub(crate) fn update_command_suggestions(&mut self) {
        self.suggestions = build_command_suggestions(self);
        if self.suggestion_index >= self.suggestions.len() {
            self.suggestion_index = 0;
        }
    }

    pub(crate) fn accept_suggestion(&mut self) {
        if let Some(s) = self.suggestions.get(self.suggestion_index) {
            self.input.set(s.fill.clone());
            self.update_command_suggestions();
        }
    }
}

fn build_command_suggestions(app: &App) -> Vec<Suggestion> {
    let raw = app.input.as_str();
    if !raw.starts_with('/') {
        return Vec::new();
    }
    let without = raw[1..].trim_start();
    let mut tokens = without.split_whitespace();
    let first = tokens.next().unwrap_or("").to_ascii_lowercase();
    let rest = tokens.collect::<Vec<_>>().join(" ");

    // Context-aware availability
    let can_done = !app.showing_projects && !app.tasks.is_empty();

    // Base commands
    let mut base: Vec<Suggestion> = vec![
        Suggestion {
            fill: String::from("/help"),
            label: String::from("❓ Help — show available commands"),
        },
        Suggestion {
            fill: String::from("/delete "),
            label: String::from("🗑️ Delete selected (or id)"),
        },
        Suggestion {
            fill: String::from("/filter"),
            label: String::from("🔍 Open the filter picker"),
        },
        Suggestion {
            fill: String::from("/refresh"),
            label: String::from("🔄 Refresh current view"),
        },
        Suggestion {
            fill: String::from("/view "),
            label: String::from("👀 Switch view (all/inbox/next/…)"),
        },
        Suggestion {
            fill: String::from("/quit"),
            label: String::from("🚪 Quit the application"),
        },
    ];

    if can_done {
        if let Some(task) = app.tasks.get(app.selected) {
            if let Some(delete) = base.iter_mut().find(|s| s.fill.starts_with("/delete")) {
                delete.fill = format!("/delete {}", task.id);
            }
        }
    }

    if rest.is_empty() {
        if first.is_empty() {
            return base;
        } else {
            return base
                .into_iter()
                .filter(|s| s.fill[1..].starts_with(&first))
                .collect();
        }
    }

    match first.as_str() {
        "view" | "tab" => {
            let partial = rest.trim().to_ascii_lowercase();
            let views = [
                ("all", "All active tasks"),
                ("inbox", "Inbox items"),
                ("next", "Next actions"),
                ("waiting", "Waiting on others"),
                ("scheduled", "Scheduled work"),
                ("someday", "Someday/Maybe"),
                ("projects", "Project health"),
                ("done", "Completed tasks"),
            ];
            views
                .iter()
                .filter_map(|(name, desc)| {
                    if partial.is_empty() || name.starts_with(&partial) {
                        Some(Suggestion {
                            fill: format!("/view {}", name),
                            label: (*desc).to_string(),
                        })
                    } else {
                        None
                    }
                })
                .collect()
        }
        "add" => {
            let entered = rest.trim();
            if entered.is_empty() {
                vec![Suggestion {
                    fill: String::from("/add "),
                    label: String::from("Enter task details…"),
                }]
            } else {
                vec![Suggestion {
                    fill: format!("/add {}", entered),
                    label: String::from("Add this task"),
                }]
            }
        }
        "done" => {
            if can_done {
                if rest.trim().is_empty() {
                    if let Some(t) = app.tasks.get(app.selected) {
                        return vec![Suggestion {
                            fill: format!("/done {}", t.id),
                            label: String::from("✅ Use selected task id"),
                        }];
                    }
                }
            }
            vec![Suggestion {
                fill: String::from("/done "),
                label: String::from("🔎 Provide a full task id"),
            }]
        }
        "delete" | "del" | "rm" => {
            if can_done {
                if rest.trim().is_empty() {
                    if let Some(t) = app.tasks.get(app.selected) {
                        return vec![Suggestion {
                            fill: format!("/delete {}", t.id),
                            label: String::from("🗑️ Use selected task id"),
                        }];
                    }
                }
            }
            vec![Suggestion {
                fill: String::from("/delete "),
                label: String::from("🔎 Provide a full task id"),
            }]
        }
        "edit" => {
            if can_done {
                if rest.trim().is_empty() {
                    if let Some(t) = app.tasks.get(app.selected) {
                        return vec![Suggestion {
                            fill: format!("/edit {} ", t.id),
                            label: String::from("✏️ Edit selected task"),
                        }];
                    }
                }
            }
            vec![Suggestion {
                fill: String::from("/edit "),
                label: String::from("✏️ Provide an id and details"),
            }]
        }
        "filter" => {
            let entered = rest.trim();
            if entered.is_empty() {
                vec![
                    Suggestion {
                        fill: String::from("/filter"),
                        label: String::from("🔍 Open the filter picker"),
                    },
                    Suggestion {
                        fill: String::from("/filter clear"),
                        label: String::from("🧹 Clear active filters"),
                    },
                ]
            } else if "clear".starts_with(&entered.to_ascii_lowercase()) {
                vec![Suggestion {
                    fill: String::from("/filter clear"),
                    label: String::from("🧹 Clear active filters"),
                }]
            } else {
                vec![Suggestion {
                    fill: String::from("/filter"),
                    label: String::from("🔍 Open the filter picker"),
                }]
            }
        }
        _ => Vec::new(),
    }
}
