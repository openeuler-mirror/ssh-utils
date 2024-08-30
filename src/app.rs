use std::io::stdout;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use anyhow::Result;
use crossterm::cursor::RestorePosition;
use crossterm::event;
use crossterm::event::Event;
use crossterm::event::KeyCode::*;
use crossterm::event::KeyEventKind;
use crossterm::event::KeyModifiers;
use crossterm::execute;
use crossterm::terminal::Clear;
use crossterm::terminal::ClearType;
use ratatui::backend::Backend;
use ratatui::buffer::Buffer;
use ratatui::layout::Constraint;
use ratatui::layout::Direction;
use ratatui::layout::Layout;
use ratatui::layout::Rect;
use ratatui::style::Color;
use ratatui::style::Modifier;
use ratatui::style::Style;
use ratatui::style::Stylize;
use ratatui::text::Text;
use ratatui::widgets::Block;
use ratatui::widgets::Borders;
use ratatui::widgets::HighlightSpacing;
use ratatui::widgets::List;
use ratatui::widgets::ListItem;
use ratatui::widgets::ListState;
use ratatui::widgets::Paragraph;
use ratatui::widgets::StatefulWidget;
use ratatui::widgets::Widget;
use ratatui::widgets::Wrap;
use ratatui::Terminal;
use russh_keys::key::KeyPair;
use russh_keys::load_secret_key;
use tokio::time::sleep;

use crate::config::app_config::Config;
use crate::config::app_vault::decrypt_password;
use crate::config::app_vault::EncryptionKey;
use crate::config::app_vault::Vault;
use crate::debug_log;
use crate::helper::convert_to_array;
use crate::ssh::key_session::KeySession;
use crate::ssh::password_session::PasswordSession;
use crate::ssh::ssh_session::{AuthMethod, SshSession};
use crate::widgets::popup_input_box::PopupInputBox;
use crate::widgets::server_creator::ServerCreator;

struct ServerItem {
    name: String,
    address: String,
    username: String,
    id: String,
    shell: String,
    port: u16,
}

struct ServerList {
    state: ListState,
    items: Vec<ServerItem>,
}

impl ServerList {
    fn with_items(items: Vec<ServerItem>) -> ServerList {
        ServerList {
            state: ListState::default(),
            items,
        }
    }

    fn next(&mut self) {
        let i = match self.state.selected() {
            Some(i) => {
                if i >= self.items.len() - 1 {
                    0
                } else {
                    i + 1
                }
            }
            None => 0,
        };
        self.state.select(Some(i));
    }

    fn previous(&mut self) {
        let i = match self.state.selected() {
            Some(i) => {
                if i == 0 {
                    self.items.len() - 1
                } else {
                    i - 1
                }
            }
            None => 0,
        };
        self.state.select(Some(i));
    }
}

pub struct PopupInfo {
    message: String,
    popup_type: PopupType,
}

#[derive(Clone)]
pub enum PopupType {
    Info,
    Error,
}

pub struct App<'a> {
    server_list: ServerList,
    vault: &'a mut Vault,
    config: &'a mut Config,
    encryption_key: EncryptionKey,
    show_popup: bool,
    popup_info: Option<PopupInfo>,
    is_connecting: bool,
}

impl<'a> Widget for &mut App<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let vertical = Layout::vertical([
            Constraint::Length(1),
            Constraint::Min(0),
            Constraint::Length(1),
        ]);
        let [head_area, body_area, foot_area] = vertical.areas(area);
        self.render_header(head_area, buf);
        self.render_servers(body_area, buf);
        self.render_footer(foot_area, buf);
    }
}

impl<'a> App<'a> {
    fn render_header(&self, area: Rect, buf: &mut Buffer) {
        let text = Text::styled(
            format!("  {:<10} {:<15} {:<20}", "user", "ip", "name"),
            Style::default().add_modifier(Modifier::BOLD),
        );
        Widget::render(text, area, buf);
    }

    fn render_footer(&self, area: Rect, buf: &mut Buffer) {
        let text = Text::from("  Add (A), Edit (E), Delete (D), Quit (ESC)").dim();
        Widget::render(text, area, buf);
    }

    fn render_servers(&mut self, area: Rect, buf: &mut Buffer) {
        let items: Vec<ListItem> = self
            .server_list
            .items
            .iter()
            .map(|item| {
                ListItem::new(format!(
                    "{:<10} {:<15} {:<20}",
                    item.username, item.address, item.name
                ))
            })
            .collect();

        let items = List::new(items)
            .highlight_style(
                Style::default()
                    .add_modifier(Modifier::BOLD)
                    .add_modifier(Modifier::REVERSED),
            )
            .highlight_symbol("> ")
            .highlight_spacing(HighlightSpacing::Always);

        StatefulWidget::render(&items, area, buf, &mut self.server_list.state);
    }
}

impl<'a> App<'a> {
    pub fn new(
        config: &'a mut Config,
        vault: &'a mut Vault,
        encryption_key: EncryptionKey,
    ) -> Result<Self> {
        let server_items: Vec<ServerItem> = config
            .servers
            .clone()
            .into_iter()
            .map(|server| ServerItem {
                id: server.id,
                name: server.name,
                address: server.ip,
                username: server.user,
                shell: server.shell,
                port: server.port,
            })
            .collect();
        let app = Self {
            server_list: ServerList::with_items(server_items),
            vault: vault,
            config: config,
            encryption_key,
            show_popup: false,
            popup_info: None,
            is_connecting: false,
        };
        Ok(app)
    }

    fn draw(&mut self, terminal: &mut Terminal<impl Backend>) -> Result<()> {
        terminal.draw(|f| {
            let show_popup = self.show_popup;
            let message = self.popup_info.as_ref().map(|info| info.message.clone());
            let title = match self.popup_info.as_ref().map(|info| info.popup_type.clone()) {
                Some(PopupType::Info) => "Info".to_string(),
                Some(PopupType::Error) => "Error".to_string(),
                None => "Info".to_string(),
            };
            let border_color = match self.popup_info.as_ref().map(|info| info.popup_type.clone()) {
                Some(PopupType::Info) => Color::LightGreen,
                Some(PopupType::Error) => Color::LightRed,
                None => Color::LightGreen,
            };
            if show_popup {
                let block = Block::default()
                    .border_style(Style::default().fg(border_color))
                    .title(title)
                    .borders(Borders::ALL);
                let area = Self::centered_rect(50, 60, f.area());
                if let Some(message) = message {
                    let text = Paragraph::new(Text::raw(message).fg(Color::White))
                        .style(Style::default())
                        .wrap(Wrap { trim: true })
                        .block(block);
                    f.render_widget(text, area);
                }
            } else {
                // we render the app itself on when there is no popup
                f.render_widget(self, f.area());
            }
        })?;
        Ok(())
    }

    /// helper function to create a centered rect using up certain percentage of the available rect `r`
    fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
        let popup_layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints(
                [
                    Constraint::Percentage((100 - percent_y) / 2),
                    Constraint::Percentage(percent_y),
                    Constraint::Percentage((100 - percent_y) / 2),
                ]
                .as_ref(),
            )
            .split(r);

        Layout::default()
            .direction(Direction::Horizontal)
            .constraints(
                [
                    Constraint::Percentage((100 - percent_x) / 2),
                    Constraint::Percentage(percent_x),
                    Constraint::Percentage((100 - percent_x) / 2),
                ]
                .as_ref(),
            )
            .split(popup_layout[1])[1]
    }

    pub async fn run(&mut self, mut terminal: &mut Terminal<impl Backend>) -> Result<()> {
        loop {
            self.draw(&mut terminal)?;
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    if !self.is_connecting && self.show_popup {
                        self.show_popup = false;
                        continue;
                    }
                    if self.is_connecting {
                        continue;
                    }
                    match key.code {
                        Char('q') | Esc => {
                            return Ok(());
                        }
                        Char('j') | Down => self.server_list.next(),
                        Char('k') | Up => self.server_list.previous(),
                        Char('c') => {
                            // Set this hotkey because of man's habit
                            if key.modifiers == KeyModifiers::CONTROL {
                                return Ok(());
                            }
                        }
                        Char('a') => {
                            // Add server
                            let mut server_creator =
                                ServerCreator::new(self.vault, self.config, &self.encryption_key);

                            if server_creator.run(&mut terminal)? {
                                self.refresh_serverlist();
                            }
                        }
                        Char('e') => {
                            // Edit server
                            if let Some(selected_index) = self.server_list.state.selected() {
                                let server = &self.server_list.items[selected_index];
                                let server_id = server.id.clone();
                                let mut server_creator = ServerCreator::new_edit(
                                    self.vault,
                                    self.config,
                                    &self.encryption_key,
                                    server_id.as_str(),
                                )?;
                                if server_creator.run(&mut terminal)? {
                                    self.refresh_serverlist();
                                }
                            }
                        }
                        Char('d') => {
                            if let Some(selected_index) = self.server_list.state.selected() {
                                let server = &self.server_list.items[selected_index];
                                let server_id = server.id.clone();
                                self.config.delete_server(server_id.as_str())?;
                                self.server_list.items.remove(selected_index);
                                self.vault.delete_server(
                                    server_id.as_str(),
                                    &convert_to_array(&self.encryption_key)?,
                                )?;
                            }
                        }
                        Enter => {
                            if let Some(selected_index) = self.server_list.state.selected() {
                                let server = &self.server_list.items[selected_index];
                                let server_id = server.id.clone();
                                let server_address = server.address.clone();
                                let server_username = server.username.clone();
                                let server_shell = server.shell.clone();
                                let server_port = server.port.clone();
                                if let Some(password) = self.vault.servers.iter().find_map(|s| {
                                    (s.id == server_id).then(|| {
                                        decrypt_password(
                                            &s.id,
                                            &s.password,
                                            &convert_to_array(&self.encryption_key).map_err(
                                                |e| anyhow::anyhow!("encryption key convert failed: {}", e),
                                            )?,
                                        )
                                        .map_err(|e| anyhow::anyhow!("password decrypt failed: {}", e))
                                    })
                                }).transpose()? {
                                    if cfg!(debug_assertions) {
                                        debug_log!("debug.log", "IP: {}", server.address);
                                        debug_log!("debug.log", "Port: {}", server.port);
                                        debug_log!("debug.log", "User: {}", server.username);
                                        debug_log!("debug.log", "Shell: {}", server.shell);
                                    }
                                    self.is_connecting = true;
                                    self.render_popup(
                                        "Connecting...".to_string(),
                                        PopupType::Info,
                                    )?;
                                    self.draw(&mut terminal)?;

                                    let is_password_empty = password.is_empty();
                                    let result: Result<Arc<dyn SshSession>, anyhow::Error> =
                                        if is_password_empty {
                                            // result 1
                                            let key_path: Option<PathBuf> = find_best_key();
                                            if key_path.is_none() {
                                                self.render_popup(
                                                    "No suitable SSH key found".to_string(),
                                                    PopupType::Error,
                                                )?;
                                                self.is_connecting = false;
                                                continue;
                                            }
                                            let key_path = key_path.unwrap(); // unwrap is safe here
                                            let key_pair: Result<KeyPair, anyhow::Error> =
                                                load_key_with_passphrase(key_path, &mut terminal);
                                            let key_pair = match key_pair {
                                                Ok(key_pair) => key_pair,
                                                Err(_) => {
                                                    self.render_popup(
                                                        "Wrong passphrase.".to_string(),
                                                        PopupType::Error,
                                                    )?;
                                                    self.is_connecting = false;
                                                    continue;
                                                }
                                            };
                                            KeySession::connect(
                                                server_username.clone(),
                                                AuthMethod::Key(key_pair),
                                                (server_address.clone(), server_port),
                                            )
                                            .await
                                            .and_then(|session| Ok(session))
                                            .map(|session| Arc::new(session) as Arc<dyn SshSession>)
                                        } else {
                                            // result 2
                                            PasswordSession::connect(
                                                server_username.clone(),
                                                AuthMethod::Password(password.clone()),
                                                (server_address.clone(), server_port),
                                            )
                                            .await
                                            .map(|session| Arc::new(session) as Arc<dyn SshSession>)
                                        };

                                    match result {
                                        Ok(mut ssh) => {
                                            self.render_popup(
                                                "Connected!".to_string(),
                                                PopupType::Info,
                                            )?;
                                            self.draw(&mut terminal)?;
                                            sleep(Duration::from_millis(1500)).await;

                                            // 处理 SSH 会话
                                            let code = {
                                                terminal.clear()?;
                                                execute!(
                                                    stdout(),
                                                    RestorePosition,
                                                    Clear(ClearType::FromCursorDown),
                                                    crossterm::cursor::Show
                                                )?;
                                                match Arc::get_mut(&mut ssh)
                                                    .unwrap()
                                                    .call(&server_shell)
                                                    .await
                                                {
                                                    Ok(code) => code,
                                                    Err(e) => {
                                                        self.render_popup(
                                                            e.to_string(),
                                                            PopupType::Error,
                                                        )?;
                                                        self.is_connecting = false;
                                                        1 // error occurred
                                                    }
                                                }
                                            };
                                            match Arc::get_mut(&mut ssh).unwrap().close().await {
                                                Ok(_) => {}
                                                Err(e) => {
                                                    self.render_popup(
                                                        e.to_string(),
                                                        PopupType::Error,
                                                    )?;
                                                    self.is_connecting = false;
                                                }
                                            }
                                            terminal.clear()?;
                                            debug_log!("debug.log", "Exitcode: {:?}", code);
                                            self.is_connecting = false;
                                            self.show_popup = false;
                                        }
                                        Err(e) => {
                                            self.show_popup = true;
                                            let error_message = if e.to_string().is_empty() {
                                                "Connection error occurred".to_string()
                                            } else {
                                                e.to_string()
                                            };
                                            debug_log!("debug.log", "{}", error_message);
                                            self.render_popup(error_message, PopupType::Error)?;
                                            self.is_connecting = false;
                                        }
                                    }
                                } else {
                                    self.render_popup(
                                        format!("Cannot find password of server {}", server.name),
                                        PopupType::Error,
                                    )?;
                                }
                            }
                        }
                        _ => {}
                    }
                }
            }
        }
    }

    fn refresh_serverlist(&mut self) {
        let server_items: Vec<ServerItem> = self
            .config
            .servers
            .clone()
            .into_iter()
            .map(|server| ServerItem {
                id: server.id,
                name: server.name,
                address: server.ip,
                username: server.user,
                shell: server.shell,
                port: server.port,
            })
            .collect();
        self.server_list = ServerList::with_items(server_items);
    }

    fn render_popup(&mut self, message: String, popup_type: PopupType) -> Result<()> {
        self.popup_info = Some(PopupInfo {
            message,
            popup_type,
        });
        self.show_popup = true;
        Ok(())
    }
}

fn find_best_key() -> Option<PathBuf> {
    let home_dir = dirs::home_dir()?;
    let ssh_dir = home_dir.join(".ssh");

    let key_priorities = [
        "id_ecdsa",     // ecdsa-sha2-nistp256
        "id_ecdsa_384", // ecdsa-sha2-nistp384
        "id_ecdsa_521", // ecdsa-sha2-nistp521
        "id_ed25519",   // ssh-ed25519
        "id_rsa",       // rsa-sha2-256, rsa-sha2-512, ssh-rsa
    ];

    for key_name in key_priorities.iter() {
        let key_path = ssh_dir.join(key_name);
        if key_path.exists() {
            return Some(key_path);
        }
    }

    None
}

fn load_key_with_passphrase(
    key_path: PathBuf,
    terminal: &mut Terminal<impl Backend>,
) -> Result<russh_keys::key::KeyPair> {
    load_secret_key(key_path.clone(), None).or_else(|e| {
        if let russh_keys::Error::KeyIsEncrypted = e {
            let mut input_box = PopupInputBox::new(" Input key's passphrase: ".to_string());
            let passphrase = input_box
                .run(terminal)?
                .ok_or_else(|| anyhow::anyhow!("Input is empty"))?;
            load_secret_key(key_path, Some(passphrase.as_str())).map_err(|e| e.into())
        } else {
            Err(e.into())
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use tempfile::TempDir;

    #[test]
    fn test_find_best_key() {
        // Create a temporary directory to simulate the home directory
        let temp_dir = TempDir::new().unwrap();
        let home_dir = temp_dir.path();
        let ssh_dir = home_dir.join(".ssh");
        std::fs::create_dir(&ssh_dir).unwrap();

        // Simulate environment variable
        std::env::set_var("HOME", home_dir.to_str().unwrap());

        // Test scenario 1: No key files present
        assert_eq!(find_best_key(), None);

        // Test scenario 2: Only id_rsa present
        File::create(ssh_dir.join("id_rsa")).unwrap();
        assert_eq!(find_best_key(), Some(ssh_dir.join("id_rsa")));

        // Test scenario 3: Both id_rsa and id_ed25519 present
        File::create(ssh_dir.join("id_ed25519")).unwrap();
        assert_eq!(find_best_key(), Some(ssh_dir.join("id_ed25519")));

        // Test scenario 4: Multiple keys present, should select the highest priority one
        File::create(ssh_dir.join("id_ecdsa")).unwrap();
        assert_eq!(find_best_key(), Some(ssh_dir.join("id_ecdsa")));

        // Cleanup
        temp_dir.close().unwrap();
    }
}
