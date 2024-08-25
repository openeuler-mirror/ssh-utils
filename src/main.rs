mod app;
mod config;
mod helper;
mod macros;
mod ssh;
mod widgets;

use anyhow::{Context, Result};
use app::App;
use clap::Parser;
use config::{
    app_config,
    app_vault::{check_if_vault_bin_exists, decrypt_vault, EncryptionKey, Vault},
    crypto::derive_key_from_password,
};
use crossterm::{
    cursor::{RestorePosition, SavePosition},
    execute,
    style::{Color, ResetColor, SetForegroundColor},
    terminal::{disable_raw_mode, enable_raw_mode, Clear, ClearType},
};
use helper::{get_file_path, ENCRYPTED_FILE};
use ratatui::{backend::CrosstermBackend, Terminal, TerminalOptions, Viewport};
use std::io::{stdout, Write};
use std::{
    fs::File,
    io::{self, Read, Stdout},
    panic::{self, PanicInfo},
};
use zeroize::Zeroize;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    /// remove all of the config file
    #[arg(short, long)]
    flush: bool,
}

fn flush_config() -> Result<()> {
    execute!(
        io::stdout(),
        SetForegroundColor(Color::Red),
        crossterm::style::Print("Warning: You are about to delete all configuration files.\n"),
        ResetColor
    )?;
    print!("Are you sure you want to continue? (y/N): ");
    io::stdout().flush()?;
    let mut input = String::new();
    io::stdin().read_line(&mut input)?;

    if input.trim().to_lowercase() == "y" {
        let mut path = dirs::home_dir().context("Unable to reach user's home directory.")?;
        path.push(".config/ssh-utils/");
        std::fs::remove_dir_all(&path).context("Failed to delete config directory")?;
        println!("Config files have been successfully deleted.");
    } else {
        println!("Operation cancelled.");
    }
    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    if cli.flush {
        flush_config()?;
        std::process::exit(0);
    }
    // Setup panic hook
    panic::set_hook(Box::new(panic_hook));
    app_config::ensure_config_exists()?;
    let mut encryption_key: EncryptionKey = Vec::with_capacity(32);
    let mut vault = init_vault(&mut encryption_key)?;
    let mut config = app_config::read_config()?;
    let app = App::new(&mut config, &mut vault, encryption_key)?;
    let mut terminal = create_terminal()?;
    setup_terminal(&mut terminal)?;
    run_app(app, &mut terminal).await?;
    restore_terminal()?;
    Ok(())
}

fn prompt_passphrase(prompt: &str) -> Result<String, anyhow::Error> {
    let prompt_password = |prompt: &str| {
        rpassword::prompt_password(prompt).or_else(|_| {
            println!("Cannot use TTY, falling back to stdin/stdout");
            println!("WARNING: Password will be visible on the screen");

            rpassword::prompt_password_from_bufread(
                &mut std::io::BufReader::new(std::io::stdin()),
                &mut std::io::stdout(),
                prompt,
            )
        })
    };

    let passphrase = prompt_password(prompt)?;
    Ok(passphrase)
}

fn init_vault(encryption_key: &mut EncryptionKey) -> Result<Vault, anyhow::Error> {
    if check_if_vault_bin_exists()? {
        for attempt in 1..=3 {
            let prompt_message = if attempt == 1 {
                "please enter a passphrase: ".to_string()
            } else {
                format!("Enter passphrase (Attempt {} of 3): ", attempt)
            };

            let mut passphrase = prompt_passphrase(&prompt_message)?;
            let try_encryption_key: [u8; 32] = derive_key_from_password(passphrase.as_str())?;
            let mut vault_file = File::open(get_file_path(ENCRYPTED_FILE)?)?;
            let mut vault_buf: Vec<u8> = Vec::new();
            vault_file.read_to_end(&mut vault_buf)?;

            // hmac challenge.
            match decrypt_vault(&vault_buf, &try_encryption_key) {
                Ok(vault) => {
                    encryption_key.extend_from_slice(&try_encryption_key);
                    // due to the drop!() is not really clear the Passphrases' data in memory.
                    // so we use zeroize to clear passphrase in memory.
                    passphrase.zeroize();
                    return Ok(vault);
                }
                Err(e) => {
                    passphrase.zeroize();
                    if let Some(_) = e.downcast_ref::<hmac::digest::MacError>() {
                        println!("Incorrect passphrase. Please try again.");
                        if attempt == 3 {
                            println!("Maximum attempts reached. Exiting.");
                            std::process::exit(1);
                        }
                    } else {
                        return Err(anyhow::anyhow!("Failed to decrypt vault: {:?}", e));
                    }
                }
            }
        }
    } else {
        execute!(
            io::stdout(),
            SetForegroundColor(Color::Green),
            crossterm::style::Print("You are the first time to use this tool.\n"),
            ResetColor
        )?;
        let mut passphrase = prompt_passphrase("Enter a passphrase to start (empty for no passphrase): ")?;
        let mut confirm_passphrase = prompt_passphrase("Enter the same passphrase again: ")?;
        if passphrase == confirm_passphrase {
            let try_encryption_key: [u8; 32] = derive_key_from_password(passphrase.as_str())?;
            passphrase.zeroize();
            confirm_passphrase.zeroize();
            encryption_key.extend_from_slice(&try_encryption_key);
            let empty_vault = Vault::default();
            empty_vault.save(&try_encryption_key)?;
            return Ok(Vault::default());
        } else {
            println!("Passphrases do not match. Please ensure both entries are identical.");
            std::process::exit(1);
        }
    }
    unreachable!()
}

async fn run_app<'a>(
    mut app: App<'a>,
    terminal: &'a mut Terminal<CrosstermBackend<Stdout>>,
) -> Result<(), anyhow::Error> {
    app.run(terminal).await?;
    Ok(())
}

fn setup_terminal(terminal: &mut Terminal<CrosstermBackend<Stdout>>) -> Result<(), anyhow::Error> {
    terminal.clear()?;
    Ok(())
}

fn create_terminal() -> Result<Terminal<CrosstermBackend<Stdout>>> {
    let mut stdout = io::stdout();
    execute!(stdout, SavePosition)?;
    enable_raw_mode()?;
    let terminal_option = TerminalOptions {
        //TODO: 设置最大行数
        viewport: Viewport::Inline(10),
    };
    Terminal::with_options(CrosstermBackend::new(stdout), terminal_option)
        .context("unable to create terminal")
}

// restore terminal to status that before exec program
fn restore_terminal() -> Result<()> {
    execute!(stdout(), RestorePosition, Clear(ClearType::FromCursorDown))?;
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
