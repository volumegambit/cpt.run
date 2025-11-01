use std::io::{self, Stdout};
use std::time::Instant;

use anyhow::{Context, Result};
use crossterm::event::{self, Event, KeyEventKind};
use crossterm::execute;
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;

use crate::config::AppConfig;
use crate::db::Database;

mod app;
mod buffer;
mod constants;
mod filters;
mod helpers;

use app::App;
use constants::TICK_RATE;

type Backend = CrosstermBackend<Stdout>;

pub fn run(config: AppConfig) -> Result<()> {
    // Check if the database file exists prior to initializing.
    let first_run = !config.db_path().exists();
    let db_path_str = config.db_path().display().to_string();

    // Ensure the database and schema exist before touching the terminal.
    let database = Database::initialize(&config)?;

    let mut stdout = io::stdout();
    enable_raw_mode().context("failed to enable raw mode")?;
    execute!(stdout, EnterAlternateScreen).context("failed to enter alternate screen")?;

    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend).context("failed to initialize terminal")?;
    terminal.hide_cursor().context("failed to hide cursor")?;

    let mut app = App::new(config, database, first_run)?;
    if first_run {
        app.set_status_info(format!(
            "Initialized cpt.run data store\n  database file: {}",
            db_path_str
        ));
    }
    let result = run_app(&mut terminal, &mut app);

    disable_raw_mode().context("failed to disable raw mode")?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)
        .context("failed to leave alternate screen")?;
    terminal.show_cursor().context("failed to show cursor")?;

    result
}

fn run_app(terminal: &mut Terminal<Backend>, app: &mut App) -> Result<()> {
    let mut last_tick = Instant::now();
    loop {
        terminal.draw(|f| app.draw(f))?;
        if app.should_quit() {
            break;
        }

        let timeout = TICK_RATE
            .checked_sub(last_tick.elapsed())
            .unwrap_or_else(|| std::time::Duration::from_secs(0));

        if event::poll(timeout)? {
            match event::read()? {
                Event::Key(key) if key.kind == KeyEventKind::Press => app.on_key(key)?,
                Event::Resize(_, _) => {}
                _ => {}
            }
        }

        if last_tick.elapsed() >= TICK_RATE {
            app.on_tick();
            last_tick = Instant::now();
        }
    }

    Ok(())
}
