//! View composition for the desktop shell, balancing productivity affordances with cpt.run visuals.

mod capture;
mod command_palette;
mod layout;
mod sidebar;
mod status;
mod styles;
mod task_table;
mod tasks;
mod toolbar;

pub(crate) use layout::compose as compose_root;
