//! Async adapters that map desktop intents into core task service calls.

use cpt_core::capture::TaskInput;
use cpt_core::model::{AddOutcome, ListFilters};
use cpt_core::TasksService;

use crate::app::message::{Effect, Message};
use crate::app::state::{MutationKind, ViewTab};

pub(crate) fn load_view_command(service: TasksService, tab: ViewTab) -> Effect {
    Effect::perform(
        async move {
            tokio::task::spawn_blocking(move || {
                let filters = ListFilters::for_view(tab.list_view());
                service.list(&filters)
            })
            .await
            .map_err(|err| err.to_string())
            .and_then(|result| result.map_err(|err| err.to_string()))
        },
        move |result| Message::ViewLoaded(tab, result),
    )
}

pub(crate) fn capture_command(
    service: TasksService,
    input: TaskInput,
) -> impl std::future::Future<Output = Result<AddOutcome, String>> {
    async move {
        tokio::task::spawn_blocking(move || service.capture(input))
            .await
            .map_err(|err| err.to_string())
            .and_then(|result| result.map_err(|err| err.to_string()))
    }
}

pub(crate) fn mutation_command(
    service: TasksService,
    kind: MutationKind,
) -> impl std::future::Future<Output = Result<(), String>> {
    async move {
        tokio::task::spawn_blocking(move || match &kind {
            MutationKind::Promote(ids) => service.promote_to_next(ids).map(|_| ()),
            MutationKind::Complete(ids) => service.mark_done(ids).map(|_| ()),
            MutationKind::Inbox(ids) => service.move_to_inbox(ids).map(|_| ()),
            MutationKind::Defer { id, until } => service.defer_until(id, Some(*until)).map(|_| ()),
            MutationKind::Rename { id, title } => service.rename_task(id, title).map(|_| ()),
            MutationKind::ChangeProject { id, project } => {
                service.update_project(id, project.clone()).map(|_| ())
            }
            MutationKind::ChangeContexts { id, contexts } => {
                service.update_contexts(id, contexts.clone()).map(|_| ())
            }
            MutationKind::ChangeTags { id, tags } => {
                service.update_tags(id, tags.clone()).map(|_| ())
            }
            MutationKind::ChangePriority { id, priority } => {
                service.update_priority(id, *priority).map(|_| ())
            }
        })
        .await
        .map_err(|err| err.to_string())
        .and_then(|result| result.map_err(|err| err.to_string()))
    }
}
