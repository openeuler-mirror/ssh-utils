use anyhow::Ok;
use anyhow::Result;
use crossterm::event;
use crossterm::event::Event;
use crossterm::event::KeyCode::*;
use crossterm::event::KeyEventKind;
use crossterm::event::KeyModifiers;
use ratatui::backend::Backend;
use ratatui::buffer::Buffer;
use ratatui::layout::Constraint;
use ratatui::layout::Layout;
use ratatui::layout::Rect;
use ratatui::style::Modifier;
use ratatui::style::Style;
use ratatui::style::Stylize;
use ratatui::text::Text;
use ratatui::widgets::HighlightSpacing;
use ratatui::widgets::List;
use ratatui::widgets::ListItem;
use ratatui::widgets::ListState;
use ratatui::widgets::StatefulWidget;
use ratatui::widgets::Widget;
use ratatui::Terminal;

use crate::config::app_config::Config;
use crate::config::app_vault::EncryptionKey;
use crate::config::app_vault::Vault;
use crate::widgets::server_creator::ServerCreator;

struct ServerItem {
    name: String,
    address: String,
    username: String,
    id: String,
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

pub struct App<'a> {
    server_list: ServerList,
    vault: &'a mut Vault,
    config: &'a mut Config,
    encryption_key: EncryptionKey,
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
            })
            .collect();
        let app = Self {
            server_list: ServerList::with_items(server_items),
            vault: vault,
            config: config,
            encryption_key,
        };
        Ok(app)
    }

    fn draw(&mut self, terminal: &mut Terminal<impl Backend>) -> Result<()> {
        terminal.draw(|f| f.render_widget(self, f.size()))?;
        Ok(())
    }

    pub fn run(&mut self, mut terminal: &mut Terminal<impl Backend>) -> Result<()> {
        loop {
            self.draw(&mut terminal)?;
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    match key.code {
                        Char('q') | Esc => return Ok(()),
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
                                    })
                                    .collect();
                                self.server_list = ServerList::with_items(server_items);
                            }
                        }
                        _ => {}
                    }
                }
            }
        }
    }
}
