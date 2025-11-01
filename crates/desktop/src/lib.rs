//! Desktop crate facade exposing the iced-based cpt.run experience to the wider workspace.

mod app;
mod telemetry;

pub use app::{run, DesktopOptions};
