use std::io::stdout;
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
use tokio::time::sleep;

use crate::config::app_config::Config;
use crate::config::app_vault::decrypt_password;
use crate::config::app_vault::EncryptionKey;
use crate::config::app_vault::Vault;
use crate::debug_log;
use crate::helper::convert_to_array;
use crate::ssh::session::Session;
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
        let text = Text::from("  Add (A), Delete (D), Quit (ESC)").dim();
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
                                // add a new server
                                // Refresh self.server_list
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
                                            &convert_to_array(&self.encryption_key).unwrap(),
                                        )
                                        .unwrap()
                                    })
                                }) {
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
                                    let mut ssh = match Session::connect(
                                        server_username.clone(),
                                        password.clone(),
                                        (server_address.clone(), server_port),
                                    )
                                    .await
                                    {
                                        Ok(ssh) => {
                                            self.render_popup(
                                                "Connected!".to_string(),
                                                PopupType::Info,
                                            )?;
                                            self.draw(&mut terminal)?;
                                            // we wait for 1.5 sec to let user know
                                            // it will be a new terminal.
                                            sleep(Duration::from_millis(1500)).await;
                                            ssh
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
                                            continue;
                                        }
                                    };
                                    // 处理 SSH 会话
                                    let code = {
                                        terminal.clear()?;
                                        execute!(
                                            stdout(),
                                            RestorePosition,
                                            Clear(ClearType::FromCursorDown),
                                            crossterm::cursor::Show
                                        )?;
                                        ssh.call(&server_shell).await?
                                    };
                                    ssh.close().await?;
                                    terminal.clear()?;
                                    debug_log!("debug.log", "Exitcode: {:?}", code);
                                    // Connect success here
                                    self.is_connecting = false;
                                    self.show_popup = false;
                                } else {
                                    self.render_popup(
                                        format!("cannt find password of server {}", server.name),
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

    fn render_popup(&mut self, message: String, popup_type: PopupType) -> Result<()> {
        self.popup_info = Some(PopupInfo {
            message,
            popup_type,
        });
        self.show_popup = true;
        Ok(())
    }
}
