use iced::alignment::Horizontal;
use iced::border::Border;
use iced::widget::{button, column, container, lazy, pick_list, row, scrollable, text, text_input};
use iced::{Alignment, Background, Color, Element, Length, Shadow};

use crate::app::message::Message;
use crate::app::state::{InlineEditState, InlineEditableField, ViewTab};
use crate::app::theme::Palette;

use super::super::desktop::CptDesktop;
use super::styles::{text_input_style, with_alpha};
use super::task_table::{
    build_project_table, build_task_table, ColumnAlignment, ProjectRow, ProjectTable, TableColumn,
    TaskRow, TaskTable,
};

impl CptDesktop {
    pub(crate) fn task_list(&self) -> Element<'_, Message> {
        let palette = self.palette;
        let active: ViewTab = self.active;
        let entry = self.views.get(&active);
        let version = entry.map(|view| view.version).unwrap_or_default();
        let snapshot = entry.and_then(|view| view.snapshot.clone());
        let selected = self.selected_task.clone();
        let inline_edit = self.inline_edit.clone();

        if let Some(snapshot) = snapshot {
            if snapshot.is_project_view() {
                let dependency = (active, version, snapshot.projects.len());
                let palette = palette;
                let snapshot_clone = snapshot.clone();
                let list = lazy(dependency, move |_| {
                    let data = build_project_table(&snapshot_clone);
                    render_project_table(data, palette)
                });

                return scrollable(list).height(Length::Fill).into();
            }

            let inline_edit_key = inline_edit
                .as_ref()
                .map(|edit| (edit.task_id.clone(), edit.value.clone(), edit.field));
            let dependency = (
                active,
                version,
                snapshot.tasks.len(),
                selected.clone(),
                inline_edit_key,
            );
            let palette = palette;
            let snapshot_clone = snapshot.clone();
            let selected_clone = selected.clone();
            let inline_edit_clone = inline_edit.clone();
            let list = lazy(dependency, move |_| {
                let data = build_task_table(active, &snapshot_clone);
                render_task_table(
                    data,
                    palette,
                    selected_clone.clone(),
                    inline_edit_clone.clone(),
                )
            });

            return scrollable(list).height(Length::Fill).into();
        }

        scrollable(column![].spacing(0)).height(Length::Fill).into()
    }
}

fn render_task_table(
    data: TaskTable,
    palette: Palette,
    selected: Option<String>,
    inline_edit: Option<InlineEditState>,
) -> Element<'static, Message> {
    let mut table = column![build_header_row(&data.columns, palette)].spacing(4);

    for row_data in data.rows {
        let is_selected = selected
            .as_ref()
            .map(|id| id == &row_data.id)
            .unwrap_or(false);
        table = table.push(build_task_row(
            &data.columns,
            row_data,
            palette,
            is_selected,
            inline_edit.clone(),
        ));
    }

    table.into()
}

fn render_project_table(data: ProjectTable, palette: Palette) -> Element<'static, Message> {
    let mut table = column![build_header_row(&data.columns, palette)].spacing(4);

    for (index, row) in data.rows.into_iter().enumerate() {
        table = table.push(build_project_row(
            &data.columns,
            row,
            palette,
            index % 2 == 1,
        ));
    }

    table.into()
}

fn build_header_row(columns: &[TableColumn], palette: Palette) -> Element<'static, Message> {
    let mut header = row![].spacing(8).align_y(Alignment::Center);
    for column in columns {
        header = header.push(
            text(column.label.to_uppercase())
                .size(12)
                .color(palette.text_secondary)
                .width(Length::FillPortion(column.portion))
                .align_x(horizontal_alignment(column.alignment)),
        );
    }

    container(header)
        .width(Length::Fill)
        .padding([6, 12])
        .style(move |_| table_header_style(palette))
        .into()
}

fn build_task_row(
    columns: &[TableColumn],
    row_data: TaskRow,
    palette: Palette,
    selected: bool,
    inline_edit: Option<InlineEditState>,
) -> Element<'static, Message> {
    let mut cells = row![].spacing(8).align_y(Alignment::Center);
    let active_edit = inline_edit.as_ref().and_then(|edit| {
        if edit.task_id == row_data.id {
            Some(edit.clone())
        } else {
            None
        }
    });

    for (column, value) in columns.iter().zip(row_data.cells.iter()) {
        let cell: Element<'static, Message> = if let Some(edit) = active_edit.as_ref() {
            match (edit.field, column.label) {
                (InlineEditableField::Title, "Title") => {
                    render_title_editor(edit.clone(), column, palette)
                }
                (InlineEditableField::Project, "Project") => {
                    render_project_editor(edit.clone(), column, palette)
                }
                (InlineEditableField::Contexts, "Contexts") => render_token_editor(
                    edit.clone(),
                    column,
                    palette,
                    "Contexts (comma separated)",
                    "Add context",
                ),
                (InlineEditableField::Tags, "Tags") => render_token_editor(
                    edit.clone(),
                    column,
                    palette,
                    "Tags (comma separated)",
                    "Add tag",
                ),
                (InlineEditableField::Priority, "Priority") => {
                    render_priority_editor(edit.clone(), column, palette)
                }
                _ => build_default_cell(column, value, palette, selected, &row_data.id),
            }
        } else {
            build_default_cell(column, value, palette, selected, &row_data.id)
        };
        cells = cells.push(cell);
    }

    container(cells)
        .width(Length::Fill)
        .padding([8, 12])
        .style(move |_| task_row_container_style(palette, selected))
        .into()
}

fn build_task_cell_button(
    column: &TableColumn,
    value: &str,
    palette: Palette,
    selected: bool,
    message: Message,
    disable_hover_bg: bool,
) -> Element<'static, Message> {
    let label = text(value.to_string())
        .size(14)
        .color(palette.text_primary)
        .width(Length::Fill)
        .align_x(horizontal_alignment(column.alignment));

    button(label)
        .width(Length::FillPortion(column.portion))
        .padding([0, 4])
        .style(move |_, status| task_cell_style(palette, selected, status, disable_hover_bg))
        .on_press(message)
        .into()
}

fn build_default_cell(
    column: &TableColumn,
    value: &str,
    palette: Palette,
    selected: bool,
    row_id: &str,
) -> Element<'static, Message> {
    let message = match column.label {
        "Title" => Message::TaskTitlePressed(row_id.to_string()),
        "Project" => Message::TaskProjectPressed(row_id.to_string()),
        "Contexts" => Message::TaskContextsPressed(row_id.to_string()),
        "Tags" => Message::TaskTagsPressed(row_id.to_string()),
        "Priority" => Message::TaskPriorityPressed(row_id.to_string()),
        _ => Message::RowSelected(row_id.to_string()),
    };
    let disable_hover_bg = column.label == "Title";
    build_task_cell_button(column, value, palette, selected, message, disable_hover_bg)
}

fn render_title_editor(
    edit: InlineEditState,
    column: &TableColumn,
    palette: Palette,
) -> Element<'static, Message> {
    let palette_copy = palette;
    let input = text_input("Task title", &edit.value)
        .id(edit.input_id.clone())
        .on_input(Message::InlineEditChanged)
        .on_submit(Message::InlineEditSubmitted)
        .padding([6, 8])
        .size(14)
        .style(move |_, status| text_input_style(palette_copy, status))
        .width(Length::Fill);

    container(input)
        .width(Length::FillPortion(column.portion))
        .into()
}

fn render_project_editor(
    edit: InlineEditState,
    column: &TableColumn,
    palette: Palette,
) -> Element<'static, Message> {
    let palette_copy = palette;
    let input = text_input("Project", &edit.value)
        .id(edit.input_id.clone())
        .on_input(Message::InlineEditChanged)
        .on_submit(Message::InlineEditSubmitted)
        .padding([6, 8])
        .size(14)
        .style(move |_, status| text_input_style(palette_copy, status))
        .width(Length::Fill);

    let mut content = column![input].spacing(4);
    if !edit.options.is_empty() {
        let selected = if edit.value.is_empty() {
            None
        } else if edit.options.iter().any(|option| option == &edit.value) {
            Some(edit.value.clone())
        } else {
            None
        };
        content = content.push(
            pick_list(
                edit.options.clone(),
                selected,
                Message::InlineEditOptionSelected,
            )
            .placeholder("Select project")
            .width(Length::Fill),
        );
    }

    container(content)
        .width(Length::FillPortion(column.portion))
        .into()
}

fn render_token_editor(
    edit: InlineEditState,
    column: &TableColumn,
    palette: Palette,
    input_placeholder: &str,
    dropdown_placeholder: &str,
) -> Element<'static, Message> {
    let palette_copy = palette;
    let input = text_input(input_placeholder, &edit.value)
        .id(edit.input_id.clone())
        .on_input(Message::InlineEditChanged)
        .on_submit(Message::InlineEditSubmitted)
        .padding([6, 8])
        .size(14)
        .style(move |_, status| text_input_style(palette_copy, status))
        .width(Length::Fill);

    let mut content = column![input].spacing(4);
    if !edit.options.is_empty() {
        content = content.push(
            pick_list(
                edit.options.clone(),
                None::<String>,
                Message::InlineEditOptionSelected,
            )
            .placeholder(dropdown_placeholder)
            .width(Length::Fill),
        );
    }

    container(content)
        .width(Length::FillPortion(column.portion))
        .into()
}

fn render_priority_editor(
    edit: InlineEditState,
    column: &TableColumn,
    _palette: Palette,
) -> Element<'static, Message> {
    let selected = if edit.value.is_empty() {
        None
    } else if edit.options.iter().any(|option| option == &edit.value) {
        Some(edit.value.clone())
    } else {
        None
    };

    let dropdown = pick_list(
        edit.options.clone(),
        selected,
        Message::InlineEditOptionSelected,
    )
    .placeholder("Select priority")
    .width(Length::Fill);

    container(dropdown)
        .width(Length::FillPortion(column.portion))
        .into()
}

fn build_project_row(
    columns: &[TableColumn],
    row: ProjectRow,
    palette: Palette,
    striped: bool,
) -> Element<'static, Message> {
    let mut cells = row![].spacing(8).align_y(Alignment::Center);

    for (column, value) in columns.iter().zip(row.cells.iter()) {
        cells = cells.push(
            text(value.clone())
                .size(14)
                .color(palette.text_primary)
                .width(Length::FillPortion(column.portion))
                .align_x(horizontal_alignment(column.alignment)),
        );
    }

    container(cells)
        .width(Length::Fill)
        .padding([8, 12])
        .style(move |_| project_row_style(palette, striped))
        .into()
}

fn horizontal_alignment(alignment: ColumnAlignment) -> Horizontal {
    match alignment {
        ColumnAlignment::Left => Horizontal::Left,
        ColumnAlignment::Right => Horizontal::Right,
    }
}

fn table_header_style(palette: Palette) -> container::Style {
    container::Style {
        background: Some(Background::Color(with_alpha(palette.surface_muted, 0.6))),
        border: Border::default(),
        shadow: Shadow::default(),
        ..container::Style::default()
    }
}

fn task_row_container_style(palette: Palette, selected: bool) -> container::Style {
    container::Style {
        background: Some(task_row_background(palette, selected)),
        border: Border::default(),
        shadow: Shadow::default(),
        ..container::Style::default()
    }
}

fn task_cell_style(
    palette: Palette,
    selected: bool,
    status: button::Status,
    disable_hover_bg: bool,
) -> button::Style {
    let mut style = button::Style {
        background: Some(Background::Color(Color::TRANSPARENT)),
        border: Border::default(),
        text_color: palette.text_primary,
        shadow: Shadow::default(),
        ..button::Style::default()
    };

    match status {
        button::Status::Hovered | button::Status::Pressed => {
            if !disable_hover_bg {
                if !selected {
                    style.background =
                        Some(Background::Color(with_alpha(palette.surface_muted, 0.35)));
                } else {
                    style.background = Some(Background::Color(with_alpha(palette.primary, 0.25)));
                }
            }
        }
        button::Status::Disabled => {
            style.text_color = with_alpha(palette.text_primary, 0.6);
        }
        button::Status::Active => {}
    }

    style
}

fn project_row_style(palette: Palette, striped: bool) -> container::Style {
    container::Style {
        background: Some(project_row_background(palette, striped)),
        border: Border::default(),
        shadow: Shadow::default(),
        ..container::Style::default()
    }
}

fn task_row_background(palette: Palette, selected: bool) -> Background {
    let color = if selected {
        with_alpha(palette.primary, 0.18)
    } else {
        with_alpha(palette.surface_muted, 0.25)
    };
    Background::Color(color)
}

fn project_row_background(palette: Palette, striped: bool) -> Background {
    let color = if striped {
        with_alpha(palette.surface_muted, 0.35)
    } else {
        with_alpha(palette.surface_muted, 0.2)
    };
    Background::Color(color)
}
