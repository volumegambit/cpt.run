//! Seeds demo tasks so the desktop shell conveys value during first-run.

use anyhow::Result;

use cpt_core::capture::TaskInput;
use cpt_core::model::ListFilters;
use cpt_core::TasksService;

use crate::app::state::SAMPLE_SEEDS;

pub(crate) fn maybe_seed_sample_data(service: &TasksService) -> Result<bool> {
    let filters = ListFilters::for_view(None);
    let snapshot = service.list(&filters)?;
    if !snapshot.tasks.is_empty() {
        return Ok(false);
    }

    for seed in SAMPLE_SEEDS {
        let mut input = TaskInput::default();
        input.text = seed
            .text
            .split_whitespace()
            .map(|piece| piece.to_string())
            .collect();
        if let Some(notes) = seed.notes {
            input.notes = Some(notes.to_string());
        }
        if let Some(status) = seed.status {
            input.status = Some(status);
        }
        service.capture(input)?;
    }
    Ok(true)
}
