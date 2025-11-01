//! Collects lightweight desktop telemetry so product tweaks can be validated during prototyping.

use parking_lot::Mutex;

#[derive(Debug, Clone)]
pub enum Event {
    AppStarted,
    ViewChanged(String),
    RefreshRequested(String),
    RefreshCompleted { view: String, count: usize },
    RefreshFailed { view: String, error: String },
    CaptureStarted,
    CaptureFinished(String),
    MutationApplied(String),
    MutationFailed { action: String, error: String },
}

pub struct Handle {
    #[cfg(feature = "telemetry")]
    events: Mutex<Vec<Event>>,
}

impl Handle {
    pub fn new() -> Self {
        Self {
            #[cfg(feature = "telemetry")]
            events: Mutex::new(Vec::new()),
        }
    }

    pub fn record(&self, event: Event) {
        #[cfg(feature = "telemetry")]
        {
            match &event {
                Event::RefreshCompleted { view, count } => {
                    tracing::debug!(
                        view = view.as_str(),
                        count,
                        "desktop telemetry refresh completed"
                    );
                }
                Event::RefreshFailed { view, error } => {
                    tracing::debug!(view = view.as_str(), error = %error, "desktop telemetry refresh failed");
                }
                Event::AppStarted => tracing::debug!("desktop telemetry app started"),
                Event::ViewChanged(view) => {
                    tracing::debug!(view = view.as_str(), "desktop telemetry view changed")
                }
                Event::RefreshRequested(view) => {
                    tracing::debug!(view = view.as_str(), "desktop telemetry refresh requested")
                }
                Event::CaptureStarted => tracing::debug!("desktop telemetry capture started"),
                Event::CaptureFinished(id) => {
                    tracing::debug!(task_id = id.as_str(), "desktop telemetry capture finished")
                }
                Event::MutationApplied(action) => tracing::debug!(
                    action = action.as_str(),
                    "desktop telemetry mutation applied"
                ),
                Event::MutationFailed { action, error } => tracing::debug!(
                    action = action.as_str(),
                    error = %error,
                    "desktop telemetry mutation failed"
                ),
            }
            self.events.lock().push(event);
        }
        #[cfg(not(feature = "telemetry"))]
        {
            let _ = event;
        }
    }

    #[cfg(test)]
    pub fn is_enabled(&self) -> bool {
        cfg!(feature = "telemetry")
    }

    #[cfg(test)]
    pub(crate) fn events_len(&self) -> usize {
        #[cfg(feature = "telemetry")]
        {
            self.events.lock().len()
        }
        #[cfg(not(feature = "telemetry"))]
        {
            0
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn records_events_counts_when_enabled() {
        let handle = Handle::new();
        handle.record(Event::RefreshCompleted {
            view: "Next".into(),
            count: 2,
        });
        if handle.is_enabled() {
            assert_eq!(handle.events_len(), 1);
        } else {
            assert_eq!(handle.events_len(), 0);
        }
    }
}
