//! Desktop application wiring that composes views, state, and services for the cpt.run experience.

pub use self::desktop::run;
pub use self::options::DesktopOptions;

mod commands;
mod desktop;
mod helpers;
mod message;
mod options;
mod seeding;
mod state;
mod theme;
mod update;
mod views;

#[cfg(test)]
mod tests;
