use std::cmp::min;

use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{
    Block, Borders, Cell, Clear, List, ListItem, ListState, Paragraph, Row, Table, Tabs, Wrap,
};
use ratatui::Frame;

use crate::model::ListView;
use crate::tui::constants::APP_VERSION;
use crate::tui::filters::{FilterColumn, FilterOverlay, PRIORITY_LEVELS};
use crate::tui::helpers::{
    accent_title, build_help_lines, centered_rect, format_opt_datetime, format_task_detail_entries,
    inset_rect, join_prefixed, short_id, BG_ACCENT, BG_BASE, BG_PANEL, FG_ACCENT,
};

use super::{App, InputMode};

impl App {
    pub(crate) fn draw(&mut self, f: &mut Frame<'_>) {
        let size = f.size();
        f.render_widget(Clear, size);
        f.render_widget(Block::default().style(Style::default().bg(BG_BASE)), size);
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1),
                Constraint::Length(3),
                Constraint::Min(5),
                Constraint::Length(2),
            ])
            .split(size);

        self.draw_header(f, chunks[0]);
        self.draw_tabs(f, chunks[1]);
        self.draw_body(f, chunks[2]);
        self.draw_footer(f, chunks[3]);

        match self.input_mode {
            InputMode::Add | InputMode::Command | InputMode::Edit => {
                self.draw_input_overlay(f, size)
            }
            InputMode::Filter => self.draw_filter_overlay(f, size),
            InputMode::Inspect => self.draw_detail_overlay(f, size),
            InputMode::Help => self.draw_help_overlay(f, size),
            InputMode::ConfirmDelete => self.draw_confirm_overlay(f, size),
            InputMode::Normal => {}
        }
    }

    fn draw_header(&self, f: &mut Frame<'_>, area: Rect) {
        let cols = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(70), Constraint::Percentage(30)])
            .split(area);

        let current = self
            .tabs
            .get(self.tab_index)
            .map(|tab| tab.description)
            .unwrap_or("Tasks");
        let mut left_spans = vec![
            Span::styled(
                format!(" cpt.run v{} ✅ ", APP_VERSION),
                Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(format!("— {}", current)),
            Span::raw("  "),
            Span::styled(
                format!("💾 {}", self.config.db_path().display()),
                Style::default().fg(Color::DarkGray),
            ),
        ];

        if let Some(summary) = self.active_filters.summary() {
            left_spans.push(Span::raw("  "));
            left_spans.push(Span::styled(
                format!("🔍 {}", summary),
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            ));
        }

        let left_line = Line::from(left_spans);
        f.render_widget(
            Paragraph::new(left_line).style(Style::default().bg(BG_BASE)),
            cols[0],
        );

        let right_line = Line::from(vec![
            Span::styled("😺 /\\_/\\ ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                "cpt",
                Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::BOLD),
            ),
        ]);
        let right_para = Paragraph::new(right_line)
            .alignment(ratatui::layout::Alignment::Right)
            .style(Style::default().bg(BG_BASE));
        f.render_widget(right_para, cols[1]);
    }

    fn draw_tabs(&self, f: &mut Frame<'_>, area: Rect) {
        let titles: Vec<Line> = self.tabs.iter().map(|tab| Line::from(tab.label)).collect();
        let tabs = Tabs::new(titles)
            .select(self.tab_index)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(accent_title("Views"))
                    .border_style(Style::default().fg(Color::DarkGray))
                    .style(Style::default().bg(BG_PANEL)),
            )
            .highlight_style(
                Style::default()
                    .fg(Color::Green)
                    .bg(BG_ACCENT)
                    .add_modifier(Modifier::BOLD),
            );
        f.render_widget(tabs, area);
    }

    fn draw_body(&mut self, f: &mut Frame<'_>, area: Rect) {
        if self.showing_projects {
            self.draw_projects(f, area);
        } else {
            self.draw_tasks(f, area);
        }
    }

    fn draw_tasks(&mut self, f: &mut Frame<'_>, area: Rect) {
        if self.tasks.is_empty() {
            let lines = self.empty_task_state();
            let block = Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::DarkGray))
                .style(Style::default().bg(BG_PANEL));
            let inner = block.inner(area);
            f.render_widget(Clear, area);
            f.render_widget(block, area);

            if inner.width == 0 || inner.height == 0 {
                return;
            }

            let width = inner.width.min(80).max(1);
            let mut height = (lines.len() as u16).saturating_add(2).min(inner.height);
            if height < 3 && inner.height >= 3 {
                height = 3;
            }
            let content_area = centered_rect(width, height, inner);
            f.render_widget(Clear, content_area);

            let paragraph = Paragraph::new(lines)
                .wrap(Wrap { trim: true })
                .alignment(ratatui::layout::Alignment::Center)
                .style(Style::default().bg(BG_PANEL));
            f.render_widget(paragraph, content_area);
            return;
        }

        let header = Row::new(vec![
            Cell::from("#️⃣ ID"),
            Cell::from("📝 Title"),
            Cell::from("🔖 Status"),
            Cell::from("📁 Project"),
            Cell::from("🧭 Contexts"),
            Cell::from("# Tags"),
            Cell::from("⏰ Due"),
            Cell::from("⭐ Pri"),
        ])
        .style(Style::default().add_modifier(Modifier::BOLD));

        let rows: Vec<Row> = self
            .tasks
            .iter()
            .map(|task| {
                Row::new(vec![
                    Cell::from(short_id(&task.id)),
                    Cell::from(task.title.clone()),
                    Cell::from(task.status.as_str()),
                    Cell::from(task.project.clone().unwrap_or_default()),
                    Cell::from(join_prefixed(&task.contexts, "@")),
                    Cell::from(join_prefixed(&task.tags, "#")),
                    Cell::from(format_opt_datetime(task.due_at.as_ref())),
                    Cell::from(task.priority.to_string()),
                ])
            })
            .collect();

        let widths = [
            Constraint::Length(8),
            Constraint::Percentage(35),
            Constraint::Length(9),
            Constraint::Percentage(15),
            Constraint::Percentage(12),
            Constraint::Percentage(12),
            Constraint::Length(12),
            Constraint::Length(4),
        ];

        let table = Table::new(rows, widths)
            .header(header)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::DarkGray))
                    .style(Style::default().bg(BG_PANEL)),
            )
            .highlight_style(
                Style::default()
                    .fg(Color::Yellow)
                    .bg(BG_ACCENT)
                    .add_modifier(Modifier::BOLD),
            )
            .highlight_symbol("▶ ");

        f.render_stateful_widget(table, area, &mut self.table_state);
    }

    fn draw_projects(&self, f: &mut Frame<'_>, area: Rect) {
        if self.projects.is_empty() {
            let lines = self.empty_project_state();
            let block = Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::DarkGray))
                .style(Style::default().bg(BG_PANEL));
            let inner = block.inner(area);
            f.render_widget(Clear, area);
            f.render_widget(block, area);

            if inner.width == 0 || inner.height == 0 {
                return;
            }

            let width = inner.width.min(80).max(1);
            let mut height = (lines.len() as u16).saturating_add(2).min(inner.height);
            if height < 3 && inner.height >= 3 {
                height = 3;
            }
            let content_area = centered_rect(width, height, inner);
            f.render_widget(Clear, content_area);

            let paragraph = Paragraph::new(lines)
                .wrap(Wrap { trim: true })
                .alignment(ratatui::layout::Alignment::Center)
                .style(Style::default().bg(BG_PANEL));
            f.render_widget(paragraph, content_area);
            return;
        }

        let header = Row::new(vec![
            Cell::from("📁 Project"),
            Cell::from("∑ Total"),
            Cell::from("⚡ Next"),
            Cell::from("⏳ Waiting"),
            Cell::from("🌱 Someday"),
        ])
        .style(Style::default().add_modifier(Modifier::BOLD));

        let rows: Vec<Row> = self
            .projects
            .iter()
            .map(|project| {
                Row::new(vec![
                    Cell::from(project.project.clone()),
                    Cell::from(project.total.to_string()),
                    Cell::from(project.next_actions.to_string()),
                    Cell::from(project.waiting.to_string()),
                    Cell::from(project.someday.to_string()),
                ])
            })
            .collect();

        let widths = [
            Constraint::Percentage(40),
            Constraint::Length(8),
            Constraint::Length(8),
            Constraint::Length(10),
            Constraint::Length(10),
        ];

        let table = Table::new(rows, widths).header(header).block(
            Block::default()
                .borders(Borders::ALL)
                .title(accent_title("Projects"))
                .border_style(Style::default().fg(Color::DarkGray))
                .style(Style::default().bg(BG_PANEL)),
        );

        f.render_widget(table, area);
    }

    fn empty_task_state(&self) -> Vec<Line<'static>> {
        let view = self.current_view();
        let heading = match view {
            None => "All clear ✨",
            Some(ListView::Inbox) => "Inbox is quiet 📥",
            Some(ListView::Next) => "No next actions yet ⚡",
            Some(ListView::Waiting) => "Nothing pending ⏳",
            Some(ListView::Scheduled) => "Nothing scheduled 📅",
            Some(ListView::Someday) => "Someday is empty 🌱",
            Some(ListView::Done) => "No wins yet ✅",
            Some(ListView::Projects) => "Projects overview",
        };

        let base_hints = [
            "Press 'a' to capture a task.",
            "Use '/' to explore commands.",
            "Press 'f' to filter by project, context, tag, or priority.",
        ];

        let mut view_hints = Vec::new();
        if matches!(view, Some(ListView::Next)) {
            view_hints.push("Promote Inbox tasks with 'n' or run '/next'.");
        }
        if matches!(view, Some(ListView::Someday)) {
            view_hints.push("Use 's' to move tasks here when they can wait.");
        }
        if matches!(view, Some(ListView::Waiting)) {
            view_hints.push("Add 'wait:Name' during capture to track follow-ups.");
        }

        let mut lines: Vec<Line<'static>> = Vec::new();
        lines.push(Line::from(vec![Span::styled(
            heading,
            Style::default()
                .fg(Color::Green)
                .add_modifier(Modifier::BOLD),
        )]));
        lines.push(Line::default());

        for hint in base_hints {
            lines.push(Line::from(vec![Span::styled(
                hint,
                Style::default()
                    .fg(Color::Gray)
                    .add_modifier(Modifier::BOLD),
            )]));
        }

        if !view_hints.is_empty() {
            lines.push(Line::default());
        }

        if !view_hints.is_empty() {
            let hint_style = Style::default()
                .fg(Color::Gray)
                .add_modifier(Modifier::BOLD);
            for hint in view_hints {
                lines.push(Line::from(vec![Span::styled(hint, hint_style)]));
            }
        }

        if self.first_run {
            lines.push(Line::default());
            let meta_style = Style::default()
                .fg(Color::DarkGray)
                .add_modifier(Modifier::BOLD);
            lines.push(Line::from(vec![Span::styled(
                format!(
                    "Your cpt.run data lives in `{}` (adjust with `--data-dir` or `CPT_DATA_DIR`).",
                    self.config.data_dir().display()
                ),
                meta_style,
            )]));
            lines.push(Line::from(vec![Span::styled(
                "Need a sandbox? Run `mise run debug` for an isolated scratchpad.",
                meta_style,
            )]));
        }

        lines
    }

    fn empty_project_state(&self) -> Vec<Line<'static>> {
        let mut lines: Vec<Line<'static>> = Vec::new();

        lines.push(Line::from(vec![Span::styled(
            "Projects overview",
            Style::default()
                .fg(Color::Green)
                .add_modifier(Modifier::BOLD),
        )]));
        lines.push(Line::default());

        if self.first_run {
            lines.push(Line::from(vec![Span::styled(
                "Projects roll up tasks that share a `+Project` token.",
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            )]));
            lines.push(Line::default());

            let help_intro_style = Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD);
            let help_highlight_style = Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD | Modifier::UNDERLINED);
            lines.push(Line::from(vec![
                Span::styled("Need commands? ", help_intro_style),
                Span::styled("/help", help_highlight_style),
                Span::styled(" shows everything.", help_intro_style),
            ]));
            lines.push(Line::default());
        }

        let hints = [
            "Capture a task with `+ProjectName` to create or link a project.",
            "Projects summarize next, waiting, and someday counts once tasks exist.",
            "Press `Tab` to return to Inbox and add a few tasks per project to get started.",
        ];

        let hint_style = Style::default()
            .fg(Color::Gray)
            .add_modifier(Modifier::BOLD);
        for hint in hints {
            lines.push(Line::from(vec![Span::styled(hint, hint_style)]));
        }

        lines
    }

    fn draw_footer(&self, f: &mut Frame<'_>, area: Rect) {
        let lines = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(1), Constraint::Length(1)])
            .split(area);

        let status_line = if let Some(status) = &self.status {
            Line::from(vec![Span::styled(status.text.clone(), status.style())])
        } else {
            Line::from(vec![Span::raw("Ready")])
        };

        f.render_widget(Paragraph::new(status_line), lines[0]);

        let mut help = match self.input_mode {
            InputMode::Normal => String::from(
                "nav: tab/shift+tab views | j/k move | q quit | overlays: enter details ℹ️ | h help ❔ | actions: a add ✚ | e edit ✏️ | n next ⚡ | s someday 🌱 | i inbox 📥 | d done ✅ | x delete 🗑️ | tools: f filter 🔍 | / command ⌨️ | r refresh 🔄",
            ),
            InputMode::Add => String::from("Enter to capture ✍️ • Esc to cancel"),
            InputMode::Command => {
                String::from("Up/Down navigate • Tab/Right complete • Enter select/run • Esc cancel")
            }
            InputMode::Filter => String::from(
                "←/→ column • ↑/↓ move • Space toggle • Enter apply • Esc cancel",
            ),
            InputMode::Edit => String::from("Enter to save ✏️ • Esc to cancel"),
            InputMode::Inspect => String::from("Enter/Esc to close ℹ️"),
            InputMode::Help => String::from("Enter/Esc to close ❔"),
            InputMode::ConfirmDelete => {
                String::from("←/→ choose • Space toggle • Enter confirm • Esc cancel")
            }
        };

        if self.input_mode == InputMode::Normal && self.first_run {
            help.push_str(" • New here? Press `a` to capture or type `/help`");
        }

        let help_line = Line::from(vec![Span::styled(
            help,
            Style::default().fg(Color::DarkGray),
        )]);
        f.render_widget(Paragraph::new(help_line), lines[1]);
    }

    fn draw_input_overlay(&self, f: &mut Frame<'_>, area: Rect) {
        let width = min(area.width.saturating_sub(10), 80);
        let base_height: u16 = 5;
        let extra_height = match self.input_mode {
            InputMode::Command => self.suggestions.len().min(6) as u16,
            InputMode::Add | InputMode::Edit => 8,
            _ => 0,
        };
        let popup_area = centered_rect(width, base_height + extra_height, area);
        f.render_widget(Clear, popup_area);
        let title = match self.input_mode {
            InputMode::Add => "➕ Add Task",
            InputMode::Command => "⌨️ Command",
            InputMode::Edit => "✏️ Edit Task",
            InputMode::Normal
            | InputMode::Filter
            | InputMode::Inspect
            | InputMode::Help
            | InputMode::ConfirmDelete => "Input",
        };
        let inner = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(3), Constraint::Min(1)])
            .split(popup_area);

        f.render_widget(Clear, inner[0]);
        let input_block = Block::default()
            .borders(Borders::ALL)
            .title(accent_title(title))
            .border_style(Style::default().fg(Color::DarkGray))
            .style(Style::default().bg(BG_PANEL));
        f.render_widget(input_block.clone(), inner[0]);
        let input_area = input_block.inner(inner[0]);
        let paragraph = Paragraph::new(self.input.as_str())
            .style(Style::default().bg(BG_PANEL))
            .wrap(Wrap { trim: false });
        f.render_widget(paragraph, input_area);

        if self.input_mode == InputMode::Command {
            let mut lines: Vec<Line> = Vec::new();
            lines.push(Line::from(vec![Span::styled(
                "Suggestions",
                Style::default()
                    .fg(Color::DarkGray)
                    .add_modifier(Modifier::BOLD),
            )]));
            for (i, s) in self.suggestions.iter().enumerate() {
                let style = if i == self.suggestion_index {
                    Style::default().fg(Color::Yellow)
                } else {
                    Style::default()
                };
                lines.push(Line::from(vec![
                    Span::styled(s.fill.as_str(), style.add_modifier(Modifier::BOLD)),
                    Span::raw("  "),
                    Span::styled(s.label.as_str(), Style::default().fg(Color::DarkGray)),
                ]));
            }
            f.render_widget(Clear, inner[1]);
            let suggestion_block = Block::default().style(Style::default().bg(BG_PANEL));
            f.render_widget(suggestion_block.clone(), inner[1]);
            let suggestion_inner = suggestion_block.inner(inner[1]);
            f.render_widget(
                Paragraph::new(lines)
                    .wrap(Wrap { trim: true })
                    .style(Style::default().bg(BG_PANEL)),
                suggestion_inner,
            );
        } else if matches!(self.input_mode, InputMode::Add | InputMode::Edit) {
            let header = Row::new(vec![
                Cell::from("Token"),
                Cell::from("Example / Description"),
            ])
            .style(
                Style::default()
                    .fg(Color::DarkGray)
                    .add_modifier(Modifier::BOLD),
            );

            let hints: [(&str, &str); 10] = [
                ("@context", "Context label (@home, @phone)"),
                ("+project", "Project name (+Website)"),
                ("#tag", "Tag (#ops)"),
                (
                    "due:DATE",
                    "Due date (today, tomorrow, fri, 2025-01-20, +3d)",
                ),
                ("defer:DATE", "Start date (tomorrow, +1w)"),
                ("t:30m", "Time estimate (minutes or 2h)"),
                ("e:low|med|high", "Energy level"),
                ("p:0-3", "Priority (0=low … 3=high)"),
                ("wait:Name", "Waiting on person/contact"),
                ("since:DATE", "Waiting since (today, +2d)"),
            ];

            let mut rows: Vec<Row> = Vec::with_capacity(hints.len() + 1);
            rows.push(header);
            for (tok, desc) in hints.iter() {
                rows.push(
                    Row::new(vec![
                        Cell::from(*tok).style(Style::default().fg(Color::Cyan)),
                        Cell::from(*desc).style(Style::default().fg(Color::DarkGray)),
                    ])
                    .height(1),
                );
            }

            let widths = [Constraint::Length(16), Constraint::Min(10)];
            f.render_widget(Clear, inner[1]);
            let hint_block = Block::default().style(Style::default().bg(BG_PANEL));
            let hint_inner = hint_block.inner(inner[1]);
            f.render_widget(hint_block, inner[1]);
            let table = Table::new(rows, widths).column_spacing(2);
            f.render_widget(table, hint_inner);
        }
    }

    fn draw_filter_overlay(&self, f: &mut Frame<'_>, area: Rect) {
        let Some(overlay) = self.filter_overlay.as_ref() else {
            return;
        };

        let width = min(area.width.saturating_sub(10), 90);
        let height = min(area.height.saturating_sub(4), 24);
        let popup_area = centered_rect(width, height, area);
        f.render_widget(Clear, popup_area);

        let columns = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(25),
                Constraint::Percentage(25),
                Constraint::Percentage(25),
                Constraint::Percentage(25),
            ])
            .split(popup_area);

        for (idx, column) in FilterColumn::ALL.into_iter().enumerate() {
            let area = columns[idx];
            self.render_filter_column(f, area, overlay, column);
        }

        let hint_area = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(2), Constraint::Min(1)])
            .split(popup_area)[1];

        let hint_lines = Line::from(vec![Span::styled(
            "Space toggles selection • Enter applies • Esc cancels • C clears all",
            Style::default()
                .fg(Color::DarkGray)
                .add_modifier(Modifier::BOLD),
        )]);
        let hint_area = inset_rect(hint_area, 1);
        f.render_widget(Clear, hint_area);
        f.render_widget(
            Paragraph::new(hint_lines)
                .wrap(Wrap { trim: true })
                .style(Style::default().bg(BG_PANEL)),
            hint_area,
        );
    }

    fn render_filter_column(
        &self,
        f: &mut Frame<'_>,
        area: Rect,
        overlay: &FilterOverlay,
        column: FilterColumn,
    ) {
        let mut items: Vec<ListItem> = Vec::new();
        let idx = column.index();
        let is_active = overlay.column == column;
        let selected_row = overlay.row_positions[idx];

        match column {
            FilterColumn::Projects => {
                let checked = overlay.working.project.is_none();
                items.push(ListItem::new(format!(
                    "[{}] {}",
                    if checked { '✓' } else { ' ' },
                    column.clear_label()
                )));
                for project in &overlay.facets.projects {
                    let badge = if overlay.working.project.as_deref() == Some(project.as_str()) {
                        '✓'
                    } else {
                        ' '
                    };
                    items.push(ListItem::new(format!("[{badge}] {project}")));
                }
            }
            FilterColumn::Contexts => {
                let checked = overlay.working.contexts.is_empty();
                items.push(ListItem::new(format!(
                    "[{}] {}",
                    if checked { '✓' } else { ' ' },
                    column.clear_label()
                )));
                for ctx in &overlay.facets.contexts {
                    let badge = if overlay.working.contexts.contains(ctx) {
                        '✓'
                    } else {
                        ' '
                    };
                    items.push(ListItem::new(format!("[{badge}] @{ctx}")));
                }
            }
            FilterColumn::Tags => {
                let checked = overlay.working.tags.is_empty();
                items.push(ListItem::new(format!(
                    "[{}] {}",
                    if checked { '✓' } else { ' ' },
                    column.clear_label()
                )));
                for tag in &overlay.facets.tags {
                    let badge = if overlay.working.tags.contains(tag) {
                        '✓'
                    } else {
                        ' '
                    };
                    items.push(ListItem::new(format!("[{badge}] #{tag}")));
                }
            }
            FilterColumn::Priority => {
                let checked = overlay.working.priority_min.is_none();
                items.push(ListItem::new(format!(
                    "[{}] {}",
                    if checked { '✓' } else { ' ' },
                    column.clear_label()
                )));
                for priority in PRIORITY_LEVELS {
                    let badge = if overlay.working.priority_min == Some(priority) {
                        '✓'
                    } else {
                        ' '
                    };
                    items.push(ListItem::new(format!("[{badge}] ≥ {priority}")));
                }
            }
        }

        if items.is_empty() {
            items.push(ListItem::new("(no options)"));
        }

        let mut state = ListState::default();
        if is_active && !items.is_empty() {
            state.select(Some(selected_row.min(items.len().saturating_sub(1))));
        }

        let display_title = if is_active {
            format!("▶ {}", column.title())
        } else {
            column.title().to_string()
        };
        let border_style = if is_active {
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::DarkGray)
        };
        let list_style = if is_active {
            Style::default().bg(BG_PANEL)
        } else {
            Style::default().fg(Color::DarkGray).bg(BG_BASE)
        };

        let list = List::new(items)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(display_title)
                    .border_style(border_style),
            )
            .style(list_style)
            .highlight_style(
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            )
            .highlight_symbol("▶ ");

        f.render_widget(Clear, area);
        f.render_stateful_widget(list, area, &mut state);
    }

    fn draw_detail_overlay(&self, f: &mut Frame<'_>, area: Rect) {
        let Some(task) = self.inspect_task.as_ref() else {
            return;
        };

        let detail_entries = format_task_detail_entries(task);
        if detail_entries.is_empty() {
            return;
        }

        let width = min(area.width.saturating_sub(20), 90).max(40);
        let content_height = detail_entries.len() as u16 + 2;
        let popup_height = content_height
            .saturating_add(4)
            .min(area.height.saturating_sub(2))
            .max(6);
        let popup_area = centered_rect(width, popup_height, area);
        f.render_widget(Clear, popup_area);

        let block = Block::default()
            .borders(Borders::ALL)
            .title(accent_title("🗒 Task Details"))
            .border_style(Style::default().fg(Color::DarkGray))
            .style(Style::default().bg(BG_PANEL));
        let inner = block.inner(popup_area);
        f.render_widget(block, popup_area);

        let detail_area = inset_rect(inner, 1);
        f.render_widget(Clear, inner);
        let rows: Vec<Row> = detail_entries
            .into_iter()
            .map(|(key, value)| {
                Row::new(vec![
                    Cell::from(key)
                        .style(Style::default().fg(FG_ACCENT).add_modifier(Modifier::BOLD)),
                    Cell::from(value),
                ])
            })
            .collect();

        let table = Table::new(rows, [Constraint::Length(14), Constraint::Min(20)])
            .block(Block::default().style(Style::default().bg(BG_PANEL)))
            .column_spacing(2);
        f.render_widget(table, detail_area);
    }

    fn draw_help_overlay(&self, f: &mut Frame<'_>, area: Rect) {
        let lines = build_help_lines();
        let width = min(area.width.saturating_sub(10), 100);
        let height = min(lines.len() as u16 + 4, area.height.saturating_sub(2)).max(10);
        let popup_area = centered_rect(width, height, area);
        f.render_widget(Clear, popup_area);

        let block = Block::default()
            .borders(Borders::ALL)
            .title(accent_title("⌨️ Keyboard Reference"))
            .border_style(Style::default().fg(Color::DarkGray))
            .style(Style::default().bg(BG_PANEL));
        let inner = block.inner(popup_area);
        f.render_widget(block, popup_area);

        let help_lines: Vec<Line> = lines
            .into_iter()
            .map(|(combo, desc)| {
                Line::from(vec![
                    Span::styled(combo, Style::default().fg(Color::Cyan)),
                    Span::raw("  "),
                    Span::raw(desc),
                ])
            })
            .collect();

        if inner.width < 3 || inner.height < 3 {
            return;
        }

        let content = inset_rect(inner, 1);
        f.render_widget(Clear, inner);
        f.render_widget(
            Paragraph::new(help_lines)
                .wrap(Wrap { trim: true })
                .style(Style::default().bg(BG_PANEL)),
            content,
        );
    }

    fn draw_confirm_overlay(&self, f: &mut Frame<'_>, area: Rect) {
        let width = min(area.width.saturating_sub(20), 60).max(40);
        let height = 8u16;
        let popup_area = centered_rect(width, height, area);
        f.render_widget(Clear, popup_area);

        let block = Block::default()
            .borders(Borders::ALL)
            .title(accent_title("🗑 Confirm Deletion"))
            .border_style(Style::default().fg(Color::Red))
            .style(Style::default().bg(BG_PANEL));
        let inner = block.inner(popup_area);
        f.render_widget(block, popup_area);

        let task_title = self
            .tasks
            .get(self.selected)
            .map(|t| t.title.as_str())
            .unwrap_or("selected task");

        let mut lines = Vec::new();
        lines.push(Line::from(vec![Span::styled(
            "This action cannot be undone.",
            Style::default().fg(Color::Red),
        )]));
        lines.push(Line::from(vec![Span::styled(
            format!("Delete '{}'?", task_title),
            Style::default().fg(Color::White),
        )]));
        lines.push(Line::default());

        let yes_style = if self.confirm_choice == super::ConfirmChoice::Yes {
            Style::default()
                .fg(Color::Black)
                .bg(Color::Red)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::Red)
        };
        let no_style = if self.confirm_choice == super::ConfirmChoice::No {
            Style::default()
                .fg(Color::Black)
                .bg(Color::Gray)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::Gray)
        };

        lines.push(Line::from(vec![
            Span::styled("  Yes  ", yes_style),
            Span::raw("    "),
            Span::styled("  No  ", no_style),
        ]));

        f.render_widget(
            Paragraph::new(lines)
                .wrap(Wrap { trim: true })
                .alignment(ratatui::layout::Alignment::Center)
                .style(Style::default().bg(BG_PANEL)),
            inset_rect(inner, 1),
        );
    }
}
