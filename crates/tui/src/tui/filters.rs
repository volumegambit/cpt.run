use std::collections::BTreeSet;

use crate::model::{ListFilters, Task};

#[derive(Debug, Clone, Default)]
pub(crate) struct ActiveFilters {
    pub(crate) project: Option<String>,
    pub(crate) contexts: BTreeSet<String>,
    pub(crate) tags: BTreeSet<String>,
    pub(crate) priority_min: Option<u8>,
}

impl ActiveFilters {
    pub(crate) fn is_empty(&self) -> bool {
        self.project.is_none()
            && self.contexts.is_empty()
            && self.tags.is_empty()
            && self.priority_min.is_none()
    }

    pub(crate) fn summary(&self) -> Option<String> {
        if self.is_empty() {
            return None;
        }

        let mut parts = Vec::new();
        if let Some(project) = &self.project {
            parts.push(format!("project:{project}"));
        }

        if !self.contexts.is_empty() {
            let joined = self
                .contexts
                .iter()
                .map(|c| format!("@{c}"))
                .collect::<Vec<_>>()
                .join(",");
            parts.push(format!("ctx:{joined}"));
        }

        if !self.tags.is_empty() {
            let joined = self
                .tags
                .iter()
                .map(|t| format!("#{t}"))
                .collect::<Vec<_>>()
                .join(",");
            parts.push(format!("tag:{joined}"));
        }

        if let Some(priority) = self.priority_min {
            parts.push(format!("priority≥{priority}"));
        }

        Some(parts.join(" | "))
    }

    pub(crate) fn apply_to(&self, filters: &mut ListFilters) {
        if let Some(project) = &self.project {
            filters.project = Some(project.clone());
        }
        if !self.contexts.is_empty() {
            filters.contexts = self.contexts.iter().cloned().collect();
        }
        if !self.tags.is_empty() {
            filters.tags = self.tags.iter().cloned().collect();
        }
        filters.priority_min = self.priority_min;
    }
}

#[derive(Debug, Default)]
pub(crate) struct FilterFacets {
    pub(crate) projects: Vec<String>,
    pub(crate) contexts: Vec<String>,
    pub(crate) tags: Vec<String>,
}

impl FilterFacets {
    pub(crate) fn from_tasks(tasks: &[Task]) -> Self {
        let mut projects = BTreeSet::new();
        let mut contexts = BTreeSet::new();
        let mut tags = BTreeSet::new();

        for task in tasks {
            if let Some(project) = task.project.as_ref().filter(|p| !p.is_empty()) {
                projects.insert(project.clone());
            }
            for ctx in &task.contexts {
                contexts.insert(ctx.clone());
            }
            for tag in &task.tags {
                tags.insert(tag.clone());
            }
        }

        Self {
            projects: projects.into_iter().collect(),
            contexts: contexts.into_iter().collect(),
            tags: tags.into_iter().collect(),
        }
    }

    pub(crate) fn ensure_selected(&mut self, active: &ActiveFilters) {
        if let Some(project) = &active.project {
            if !self.projects.contains(project) {
                self.projects.push(project.clone());
                self.projects.sort();
            }
        }

        for ctx in &active.contexts {
            if !self.contexts.contains(ctx) {
                self.contexts.push(ctx.clone());
            }
        }
        if !self.contexts.is_empty() {
            self.contexts.sort();
        }

        for tag in &active.tags {
            if !self.tags.contains(tag) {
                self.tags.push(tag.clone());
            }
        }
        if !self.tags.is_empty() {
            self.tags.sort();
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum FilterColumn {
    Projects,
    Contexts,
    Tags,
    Priority,
}

impl FilterColumn {
    pub(crate) const ALL: [Self; 4] = [
        FilterColumn::Projects,
        FilterColumn::Contexts,
        FilterColumn::Tags,
        FilterColumn::Priority,
    ];

    pub(crate) fn index(self) -> usize {
        match self {
            FilterColumn::Projects => 0,
            FilterColumn::Contexts => 1,
            FilterColumn::Tags => 2,
            FilterColumn::Priority => 3,
        }
    }

    pub(crate) fn title(self) -> &'static str {
        match self {
            FilterColumn::Projects => "Projects",
            FilterColumn::Contexts => "Contexts",
            FilterColumn::Tags => "Tags",
            FilterColumn::Priority => "Priority",
        }
    }

    pub(crate) fn clear_label(self) -> &'static str {
        match self {
            FilterColumn::Projects => "All projects",
            FilterColumn::Contexts => "Clear contexts",
            FilterColumn::Tags => "Clear tags",
            FilterColumn::Priority => "All priorities",
        }
    }
}

#[derive(Debug)]
pub(crate) struct FilterOverlay {
    pub(crate) facets: FilterFacets,
    pub(crate) working: ActiveFilters,
    pub(crate) initial: ActiveFilters,
    pub(crate) column: FilterColumn,
    pub(crate) row_positions: [usize; 4],
}

impl FilterOverlay {
    pub(crate) fn new(mut facets: FilterFacets, active: &ActiveFilters) -> Self {
        facets.ensure_selected(active);
        Self {
            facets,
            working: active.clone(),
            initial: active.clone(),
            column: FilterColumn::Projects,
            row_positions: [0, 0, 0, 0],
        }
    }

    pub(crate) fn next_column(&mut self) {
        let idx = self.column.index();
        let next = (idx + 1) % FilterColumn::ALL.len();
        self.column = FilterColumn::ALL[next];
        self.clamp_rows();
    }

    pub(crate) fn prev_column(&mut self) {
        let idx = self.column.index();
        let prev = if idx == 0 {
            FilterColumn::ALL.len() - 1
        } else {
            idx - 1
        };
        self.column = FilterColumn::ALL[prev];
        self.clamp_rows();
    }

    pub(crate) fn next_row(&mut self) {
        let max = self.current_len().saturating_sub(1);
        let row = &mut self.row_positions[self.column.index()];
        if *row >= max {
            *row = 0;
        } else {
            *row += 1;
        }
    }

    pub(crate) fn prev_row(&mut self) {
        let max = self.current_len().saturating_sub(1);
        let row = &mut self.row_positions[self.column.index()];
        if *row == 0 {
            *row = max;
        } else {
            *row -= 1;
        }
    }

    pub(crate) fn toggle_current(&mut self) {
        match self.column {
            FilterColumn::Projects => {
                let row = self.row_positions[FilterColumn::Projects.index()];
                if row == 0 {
                    self.working.project = None;
                } else if let Some(project) = self.facets.projects.get(row - 1) {
                    if self.working.project.as_deref() == Some(project.as_str()) {
                        self.working.project = None;
                    } else {
                        self.working.project = Some(project.clone());
                    }
                }
            }
            FilterColumn::Contexts => {
                let row = self.row_positions[FilterColumn::Contexts.index()];
                if row == 0 {
                    self.working.contexts.clear();
                } else if let Some(ctx) = self.facets.contexts.get(row - 1) {
                    if self.working.contexts.contains(ctx) {
                        self.working.contexts.remove(ctx);
                    } else {
                        self.working.contexts.insert(ctx.clone());
                    }
                }
            }
            FilterColumn::Tags => {
                let row = self.row_positions[FilterColumn::Tags.index()];
                if row == 0 {
                    self.working.tags.clear();
                } else if let Some(tag) = self.facets.tags.get(row - 1) {
                    if self.working.tags.contains(tag) {
                        self.working.tags.remove(tag);
                    } else {
                        self.working.tags.insert(tag.clone());
                    }
                }
            }
            FilterColumn::Priority => {
                let row = self.row_positions[FilterColumn::Priority.index()];
                if row == 0 {
                    self.working.priority_min = None;
                } else if let Some(priority) = PRIORITY_LEVELS.get(row - 1) {
                    if self.working.priority_min == Some(*priority) {
                        self.working.priority_min = None;
                    } else {
                        self.working.priority_min = Some(*priority);
                    }
                }
            }
        }
    }

    pub(crate) fn clear_all(&mut self) {
        self.working = ActiveFilters::default();
        self.row_positions = [0, 0, 0, 0];
    }

    pub(crate) fn cancel(mut self) -> ActiveFilters {
        self.working = self.initial;
        self.working
    }

    pub(crate) fn commit(self) -> ActiveFilters {
        self.working
    }

    pub(crate) fn current_len(&self) -> usize {
        match self.column {
            FilterColumn::Projects => 1 + self.facets.projects.len(),
            FilterColumn::Contexts => 1 + self.facets.contexts.len(),
            FilterColumn::Tags => 1 + self.facets.tags.len(),
            FilterColumn::Priority => 1 + PRIORITY_LEVELS.len(),
        }
    }

    fn clamp_rows(&mut self) {
        let len = self.current_len();
        let row = &mut self.row_positions[self.column.index()];
        if *row >= len {
            *row = len.saturating_sub(1);
        }
    }
}

pub(crate) const PRIORITY_LEVELS: [u8; 4] = [0, 1, 2, 3];
