//! Standalone entry point for developers to launch the desktop prototype without the CLI wrapper.

fn main() -> iced::Result {
    cpt_desktop::run(cpt_desktop::DesktopOptions::default())
}
