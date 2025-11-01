use std::collections::HashSet;

use anyhow::{anyhow, Context, Result};
use chrono::{prelude::*, Duration, Months};
use once_cell::sync::Lazy;
use regex::Regex;

use crate::capture::CaptureInput;
use crate::model::{AddOutcome, EnergyLevel, InsertableTask, NewTask, TaskStatus};

#[derive(Debug, Clone)]
pub struct ParsedTask {
    pub task: NewTask,
    pub title: String,
    pub status: TaskStatus,
}

/// Result of inline token parsing from the capture text.
#[derive(Debug, Default)]
struct InlineTokens {
    title_words: Vec<String>,
    project: Option<String>,
    contexts: Vec<String>,
    tags: Vec<String>,
    due_at: Option<DateTime<Utc>>,
    defer_until: Option<DateTime<Utc>>,
    time_estimate: Option<u32>,
    energy: Option<EnergyLevel>,
    priority: Option<u8>,
    waiting_on: Option<String>,
    waiting_since: Option<DateTime<Utc>>,
}

pub fn prepare_new_task(input: &CaptureInput) -> Result<(InsertableTask, AddOutcome)> {
    let parsed = parse_capture(input)?;
    let insertable = parsed.task.clone().into_insertable();
    let outcome = AddOutcome {
        id: insertable.id.clone(),
        status: parsed.status,
        title: parsed.title,
    };

    Ok((insertable, outcome))
}

pub fn parse_capture(input: &CaptureInput) -> Result<ParsedTask> {
    let raw_text = input.text.join(" ");
    let inline = parse_inline_tokens(&raw_text)?;

    let mut contexts = merge_lists(inline.contexts, normalize_labels(&input.contexts));
    let mut tags = merge_lists(inline.tags, normalize_labels(&input.tags));
    let mut areas = normalize_labels(&input.areas);

    contexts.sort();
    contexts.dedup();
    tags.sort();
    tags.dedup();
    areas.sort();
    areas.dedup();

    let project = input
        .project
        .as_ref()
        .map(|s| s.trim().to_string())
        .or(inline.project);

    let priority = match input.priority {
        Some(p) => p.min(3),
        None => inline.priority.unwrap_or(0),
    };

    let energy = if let Some(label) = input.energy.as_ref() {
        Some(label.parse::<EnergyLevel>()?)
    } else {
        inline.energy
    };

    let due_at = match &input.due_at {
        Some(spec) => Some(parse_date_spec(spec)?),
        None => inline.due_at,
    };

    let defer_until = match &input.defer_until {
        Some(spec) => Some(parse_date_spec(spec)?),
        None => inline.defer_until,
    };

    let time_estimate = input.time_estimate.or(inline.time_estimate);

    let waiting_on = input
        .waiting_on
        .as_ref()
        .map(|s| s.trim().to_string())
        .or(inline.waiting_on);

    let waiting_since = match &input.waiting_since {
        Some(spec) => Some(parse_date_spec(spec)?),
        None => inline.waiting_since,
    };

    let status = input
        .status
        .unwrap_or_else(|| TaskStatus::default_for_waiting(waiting_on.is_some()));

    let title = inline.title_words.join(" ").trim().to_string();
    let title = if title.is_empty() {
        raw_text.trim().to_string()
    } else {
        title
    };

    if title.is_empty() {
        return Err(anyhow!("Task title cannot be empty after parsing tokens"));
    }

    let task = NewTask {
        title: title.clone(),
        notes: input.notes.clone(),
        status,
        project,
        areas,
        contexts,
        tags,
        priority,
        energy,
        time_estimate,
        due_at,
        defer_until,
        repeat: None,
        waiting_on,
        waiting_since,
    };

    Ok(ParsedTask {
        task,
        title,
        status,
    })
}

fn parse_inline_tokens(text: &str) -> Result<InlineTokens> {
    let mut result = InlineTokens::default();
    let mut waiting_since_token: Option<DateTime<Utc>> = None;

    for raw_piece in text.split_whitespace() {
        let (piece, trailing) = strip_trailing_punctuation(raw_piece);
        if piece.starts_with('@') && piece.len() > 1 {
            result
                .contexts
                .push(normalize_label(piece.trim_start_matches('@')));
            if let Some(rest) = trailing {
                push_trailing(&mut result.title_words, rest);
            }
            continue;
        }
        if piece.starts_with('+') && piece.len() > 1 {
            result.project = Some(clean_title(piece.trim_start_matches('+')));
            if let Some(rest) = trailing {
                push_trailing(&mut result.title_words, rest);
            }
            continue;
        }
        if piece.starts_with('#') && piece.len() > 1 {
            result
                .tags
                .push(normalize_label(piece.trim_start_matches('#')));
            if let Some(rest) = trailing {
                push_trailing(&mut result.title_words, rest);
            }
            continue;
        }
        if let Some(spec) = piece.strip_prefix("due:") {
            result.due_at = Some(parse_date_spec(spec)?);
            if let Some(rest) = trailing {
                push_trailing(&mut result.title_words, rest);
            }
            continue;
        }
        if let Some(spec) = piece.strip_prefix("defer:") {
            result.defer_until = Some(parse_date_spec(spec)?);
            if let Some(rest) = trailing {
                push_trailing(&mut result.title_words, rest);
            }
            continue;
        }
        if let Some(spec) = piece.strip_prefix("t:") {
            result.time_estimate = Some(parse_duration_minutes(spec)?);
            if let Some(rest) = trailing {
                push_trailing(&mut result.title_words, rest);
            }
            continue;
        }
        if let Some(spec) = piece.strip_prefix("e:") {
            result.energy = Some(spec.parse::<EnergyLevel>()?);
            if let Some(rest) = trailing {
                push_trailing(&mut result.title_words, rest);
            }
            continue;
        }
        if let Some(spec) = piece.strip_prefix("p:") {
            result.priority = Some(spec.parse::<u8>()?.min(3));
            if let Some(rest) = trailing {
                push_trailing(&mut result.title_words, rest);
            }
            continue;
        }
        if let Some(spec) = piece.strip_prefix("wait:") {
            if !spec.is_empty() {
                result.waiting_on = Some(clean_title(spec));
            }
            if let Some(rest) = trailing {
                push_trailing(&mut result.title_words, rest);
            }
            continue;
        }
        if let Some(spec) = piece.strip_prefix("since:") {
            waiting_since_token = Some(parse_date_spec(spec)?);
            if let Some(rest) = trailing {
                push_trailing(&mut result.title_words, rest);
            }
            continue;
        }

        result.title_words.push(raw_piece.to_string());
    }

    if result.waiting_since.is_none() {
        result.waiting_since = waiting_since_token;
    }

    Ok(result)
}

pub fn normalize_labels(values: &[String]) -> Vec<String> {
    values
        .iter()
        .map(|v| normalize_label(v))
        .filter(|v| !v.is_empty())
        .collect()
}

fn normalize_label(value: &str) -> String {
    let stripped = value.trim().trim_start_matches(&['@', '#'][..]);
    stripped.to_ascii_lowercase()
}

fn merge_lists(mut primary: Vec<String>, secondary: Vec<String>) -> Vec<String> {
    let mut seen: HashSet<String> = primary.iter().cloned().collect();
    for value in secondary {
        if seen.insert(value.clone()) {
            primary.push(value);
        }
    }
    primary
}

fn parse_duration_minutes(spec: &str) -> Result<u32> {
    let spec = spec.trim().to_ascii_lowercase();
    if spec.ends_with("m") {
        let number = &spec[..spec.len() - 1];
        Ok(number.parse::<u32>()?)
    } else if spec.ends_with("h") {
        let number = &spec[..spec.len() - 1];
        Ok(number.parse::<u32>()?.saturating_mul(60))
    } else {
        Ok(spec.parse::<u32>()?)
    }
}

fn strip_trailing_punctuation(input: &str) -> (String, Option<String>) {
    static PUNCT_RE: Lazy<Regex> = Lazy::new(|| Regex::new(r"[[:punct:]]+$").expect("valid regex"));
    if let Some(mat) = PUNCT_RE.find(input) {
        let token = input[..mat.start()].to_string();
        let trailing = input[mat.start()..].to_string();
        (token, Some(trailing))
    } else {
        (input.to_string(), None)
    }
}

fn clean_title(value: &str) -> String {
    value
        .trim_matches(|c: char| c == ',' || c == ';' || c == '.')
        .trim()
        .to_string()
}

fn push_trailing(words: &mut Vec<String>, trailing: String) {
    if let Some(last) = words.last_mut() {
        last.push_str(&trailing);
    } else {
        words.push(trailing);
    }
}

pub fn parse_date_spec(spec: &str) -> Result<DateTime<Utc>> {
    let trimmed = spec.trim();
    if trimmed.is_empty() {
        return Err(anyhow!("Date specification cannot be empty"));
    }

    let lower = trimmed.to_ascii_lowercase();
    let now_local = Local::now();

    match lower.as_str() {
        "now" => return Ok(now_local.with_timezone(&Utc)),
        "today" => {
            let today = now_local.date_naive();
            let dt = today
                .and_hms_opt(9, 0, 0)
                .unwrap_or_else(|| today.and_hms_opt(0, 0, 0).expect("midnight"));
            return Ok(Local
                .from_local_datetime(&dt)
                .single()
                .expect("local today")
                .with_timezone(&Utc));
        }
        "tomorrow" => {
            let date = now_local.date_naive() + Duration::days(1);
            let dt = date
                .and_hms_opt(9, 0, 0)
                .unwrap_or_else(|| date.and_hms_opt(0, 0, 0).expect("midnight"));
            return Ok(Local
                .from_local_datetime(&dt)
                .single()
                .expect("local tomorrow")
                .with_timezone(&Utc));
        }
        _ => {}
    }

    if lower.starts_with('+') {
        return parse_relative_spec(&lower, now_local);
    }

    if let Some(weekday) = parse_weekday(&lower) {
        let mut days_ahead = (weekday.num_days_from_monday() as i32
            - now_local.weekday().num_days_from_monday() as i32)
            .rem_euclid(7);
        if days_ahead == 0 {
            days_ahead = 7;
        }
        let target = now_local + Duration::days(days_ahead.into());
        let date = target.date_naive();
        let dt = date
            .and_hms_opt(9, 0, 0)
            .unwrap_or_else(|| date.and_hms_opt(0, 0, 0).expect("midnight"));
        return Ok(Local
            .from_local_datetime(&dt)
            .single()
            .expect("local weekday")
            .with_timezone(&Utc));
    }

    if let Ok(parsed) = DateTime::parse_from_rfc3339(trimmed) {
        return Ok(parsed.with_timezone(&Utc));
    }

    if let Ok(date) = NaiveDate::parse_from_str(trimmed, "%Y-%m-%d") {
        let dt = date
            .and_hms_opt(9, 0, 0)
            .unwrap_or_else(|| date.and_hms_opt(0, 0, 0).expect("midnight"));
        return Ok(Local
            .from_local_datetime(&dt)
            .single()
            .expect("local date")
            .with_timezone(&Utc));
    }

    if let Ok(time) = NaiveTime::parse_from_str(trimmed, "%H:%M") {
        let date = now_local.date_naive();
        let dt = date.and_time(time);
        return Ok(Local
            .from_local_datetime(&dt)
            .single()
            .ok_or_else(|| anyhow!("Could not resolve local time for '{}'", trimmed))?
            .with_timezone(&Utc));
    }

    Err(anyhow!(
        "Unrecognized date specification '{}'. Try YYYY-MM-DD, today, tomorrow, +3d, mon",
        spec
    ))
}

fn parse_relative_spec(spec: &str, now_local: DateTime<Local>) -> Result<DateTime<Utc>> {
    if spec.len() < 3 {
        return Err(anyhow!("Relative date '{}' is too short", spec));
    }
    let (number_part, unit) = spec[1..].split_at(spec.len() - 2);
    let value: i64 = number_part.parse().context("Invalid relative offset")?;
    match unit {
        "d" => Ok((now_local + Duration::days(value)).with_timezone(&Utc)),
        "w" => Ok((now_local + Duration::weeks(value)).with_timezone(&Utc)),
        "m" => {
            let months = Months::new(value.try_into()?);
            Ok((now_local + months).with_timezone(&Utc))
        }
        other => Err(anyhow!(
            "Unsupported relative unit '{}'. Use d, w, or m.",
            other
        )),
    }
}

fn parse_weekday(label: &str) -> Option<Weekday> {
    match label {
        "mon" | "monday" => Some(Weekday::Mon),
        "tue" | "tuesday" => Some(Weekday::Tue),
        "wed" | "wednesday" => Some(Weekday::Wed),
        "thu" | "thursday" => Some(Weekday::Thu),
        "fri" | "friday" => Some(Weekday::Fri),
        "sat" | "saturday" => Some(Weekday::Sat),
        "sun" | "sunday" => Some(Weekday::Sun),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::capture::CaptureInput;
    use pretty_assertions::assert_eq;

    #[test]
    fn parses_basic_tokens() {
        let add = CaptureInput {
            text: vec![
                "Email".into(),
                "Alice".into(),
                "@work".into(),
                "+Sales".into(),
                "due:tomorrow".into(),
            ],
            notes: None,
            project: None,
            areas: vec![],
            status: None,
            contexts: vec![],
            tags: vec!["#q4".into()],
            due_at: None,
            defer_until: None,
            time_estimate: Some(15),
            energy: Some("low".into()),
            priority: Some(2),
            waiting_on: None,
            waiting_since: None,
        };

        let (task, outcome) = prepare_new_task(&add).unwrap();
        assert_eq!(outcome.title, "Email Alice");
        assert_eq!(outcome.status.as_str(), "inbox");
        assert!(task.data.project.is_some());
        assert!(task.data.due_at.is_some());
        assert_eq!(task.data.contexts, vec!["work"]);
        assert_eq!(task.data.tags, vec!["q4"]);
    }

    #[test]
    fn parses_time_shorthand() {
        assert_eq!(parse_duration_minutes("30").unwrap(), 30);
        assert_eq!(parse_duration_minutes("45m").unwrap(), 45);
        assert_eq!(parse_duration_minutes("2h").unwrap(), 120);
    }

    #[test]
    fn waiting_token_defaults_status() {
        let add = CaptureInput {
            text: vec!["Check".into(), "status".into(), "wait:Alice".into()],
            notes: None,
            project: None,
            areas: vec![],
            status: None,
            contexts: vec![],
            tags: vec![],
            due_at: None,
            defer_until: None,
            time_estimate: None,
            energy: None,
            priority: None,
            waiting_on: None,
            waiting_since: None,
        };

        let (_task, outcome) = prepare_new_task(&add).unwrap();
        assert_eq!(outcome.status, TaskStatus::Waiting);
    }

    #[test]
    fn explicit_status_overrides_waiting_default() {
        let add = CaptureInput {
            text: vec!["Check".into(), "status".into(), "wait:Alice".into()],
            notes: None,
            project: None,
            areas: vec![],
            status: Some(TaskStatus::Next),
            contexts: vec![],
            tags: vec![],
            due_at: None,
            defer_until: None,
            time_estimate: None,
            energy: None,
            priority: None,
            waiting_on: None,
            waiting_since: None,
        };

        let (_task, outcome) = prepare_new_task(&add).unwrap();
        assert_eq!(outcome.status, TaskStatus::Next);
    }

    #[test]
    fn parses_iso_due_dates() {
        let add = CaptureInput {
            text: vec!["Submit".into(), "report".into(), "due:2025-12-24".into()],
            notes: None,
            project: None,
            areas: vec![],
            status: None,
            contexts: vec![],
            tags: vec![],
            due_at: None,
            defer_until: None,
            time_estimate: None,
            energy: None,
            priority: None,
            waiting_on: None,
            waiting_since: None,
        };

        let (task, outcome) = prepare_new_task(&add).unwrap();
        assert_eq!(outcome.title, "Submit report");
        let date = task.data.due_at.expect("due date");
        assert_eq!(date.date_naive().to_string(), "2025-12-24");
    }
}
