mod app;
mod widgets;
mod config;

use std::{io::{self, Stdout}, panic::{self, PanicInfo}};
use app::App;
use config::app_config;
use crossterm::{cursor::{RestorePosition, SavePosition}, execute, terminal::{
		disable_raw_mode, enable_raw_mode, Clear, ClearType
	}};
use ratatui::{backend::CrosstermBackend, Terminal, TerminalOptions, Viewport};
use anyhow::{Context, Result};
use std::io::stdout;

fn main() -> Result<()> {
    // Setup panic hook
    panic::set_hook(Box::new(panic_hook));
    app_config::ensure_config_exists()?;
    let config = app_config::read_config()?;
    let app = App::new(config)?;
    let mut terminal = create_terminal()?;
    setup_terminal(&mut terminal)?;
    run_app(app, &mut terminal)?;
    restore_terminal()?;
    Ok(())
}

fn run_app(mut app: App, terminal: &mut Terminal<CrosstermBackend<Stdout>>) -> Result<(), anyhow::Error> {
    app.run(terminal)?;
    Ok(())
}

fn setup_terminal(terminal: &mut Terminal<CrosstermBackend<Stdout>>) -> Result<(), anyhow::Error> {
    terminal.clear()?;
    Ok(())
}

fn create_terminal() -> Result<Terminal<CrosstermBackend<Stdout>>> {
    let mut stdout = io::stdout();
    execute!(
        stdout,
        SavePosition
    )?;
    enable_raw_mode()?;
    let terminal_option = TerminalOptions {
        //TODO: 设置最大行数
        viewport: Viewport::Inline(10)
    };
    Terminal::with_options(CrosstermBackend::new(stdout), terminal_option).context("unable to create terminal")
}

// restore terminal to status that before exec program
fn restore_terminal() -> Result<()> {
    execute!(
        stdout(),
        RestorePosition,
        Clear(ClearType::FromCursorDown)
    )?;
    disable_raw_mode()?;
    Ok(())
}

/// A panic hook to properly restore the terminal in the case of a panic.
/// Originally based on [spotify-tui's implementation](https://github.com/Rigellute/spotify-tui/blob/master/src/main.rs).
fn panic_hook(panic_info: &PanicInfo<'_>) {
    let mut stdout = stdout();

    let msg = match panic_info.payload().downcast_ref::<&'static str>() {
        Some(s) => *s,
        None => match panic_info.payload().downcast_ref::<String>() {
            Some(s) => &s[..],
            None => "Box<Any>",
        },
    };

    let backtrace = format!("{:?}", backtrace::Backtrace::new());

    if let Err(e) = restore_terminal() {
        eprintln!("unable to restore terminal:\n{e}");
    }

    // Print stack trace. Must be done after!
    if let Some(panic_info) = panic_info.location() {
        let _ = execute!(
            stdout,
            crossterm::style::Print(format!(
                "application panic: '{msg}', {panic_info}\n\r{backtrace}",
            )),
        );
    }
}