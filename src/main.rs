mod app;

use std::{io::{self, Stdout}, panic};
use app::App;
use backtrace::Backtrace;
use crossterm::terminal::{
		disable_raw_mode, enable_raw_mode
	};
use ratatui::{backend::CrosstermBackend, Terminal, TerminalOptions, Viewport};
use anyhow::{Context, Ok, Result};

fn main() -> Result<()> {
    set_panic_handlers()?;
    let mut terminal = create_terminal()?;

    setup_terminal(&mut terminal)?;
    let app = App::new()?;
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
    let stdout = io::stdout();
    enable_raw_mode()?;
    let terminal_option = TerminalOptions {
        //TODO: 设置最大行数
        viewport: Viewport::Inline(10)
    };
    Terminal::with_options(CrosstermBackend::new(stdout), terminal_option).context("unable to create terminal")
}

fn restore_terminal() -> Result<()> {
    disable_raw_mode()?;
    Ok(())
}

// handle all panic here
fn set_panic_handlers() -> Result<()> {
	panic::set_hook(Box::new(|e| {
        if let Err(e) = disable_raw_mode() {
            eprintln!("unable to disable raw mode:\n{e}");
        }
		let backtrace = Backtrace::new();
		eprintln!("\nssh-utils was close due to an unexpected panic with the following info:\n\n{:?}\ntrace:\n{:?}", e, backtrace);
	}));
	Ok(())
}