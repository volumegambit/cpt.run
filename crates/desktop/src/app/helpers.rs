//! Helper utilities for detecting environment defaults and previewing capture tokens.

use chrono::{DateTime, Local, Utc};
use cpt_core::capture::CaptureInput;
use dark_light::Mode as ThemePreference;
use iced::Theme;

use crate::app::state::{CaptureChip, CaptureChipKind, CapturePreview};

pub(crate) fn detect_theme() -> Theme {
    match dark_light::detect() {
        ThemePreference::Dark => Theme::Dark,
        ThemePreference::Light => Theme::Light,
        ThemePreference::Default => Theme::Dark,
    }
}

pub(crate) fn capture_preview(input: &str) -> Result<Option<CapturePreview>, String> {
    if input.trim().is_empty() {
        return Ok(None);
    }
    let capture = CaptureInput {
        text: input
            .split_whitespace()
            .map(|piece| piece.to_string())
            .collect(),
        ..CaptureInput::default()
    };

    match cpt_core::parser::parse_capture(&capture) {
        Ok(parsed) => {
            let mut chips = Vec::new();
            if let Some(project) = parsed.task.project.as_ref() {
                chips.push(CaptureChip {
                    label: project.clone(),
                    kind: CaptureChipKind::Project,
                });
            }
            for ctx in &parsed.task.contexts {
                chips.push(CaptureChip {
                    label: ctx.clone(),
                    kind: CaptureChipKind::Context,
                });
            }
            for tag in &parsed.task.tags {
                chips.push(CaptureChip {
                    label: tag.clone(),
                    kind: CaptureChipKind::Tag,
                });
            }
            if let Some(due) = parsed.task.due_at {
                chips.push(CaptureChip {
                    label: format_datetime(due),
                    kind: CaptureChipKind::Due,
                });
            }
            if let Some(defer) = parsed.task.defer_until {
                chips.push(CaptureChip {
                    label: format_datetime(defer),
                    kind: CaptureChipKind::Defer,
                });
            }
            if let Some(waiting) = parsed.task.waiting_on.as_ref() {
                chips.push(CaptureChip {
                    label: waiting.clone(),
                    kind: CaptureChipKind::Waiting,
                });
            }
            if let Some(energy) = parsed.task.energy {
                chips.push(CaptureChip {
                    label: energy.as_str().to_string(),
                    kind: CaptureChipKind::Energy,
                });
            }
            if parsed.task.priority > 0 {
                chips.push(CaptureChip {
                    label: parsed.task.priority.to_string(),
                    kind: CaptureChipKind::Priority,
                });
            }
            Ok(Some(CapturePreview { chips }))
        }
        Err(err) => Err(err.to_string()),
    }
}

pub(crate) fn format_datetime(dt: DateTime<Utc>) -> String {
    let local = dt.with_timezone(&Local);
    local.format("%a %b %d %H:%M").to_string()
}

pub(crate) fn capitalize(text: &str) -> String {
    let mut chars = text.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
        None => String::new(),
    }
}
