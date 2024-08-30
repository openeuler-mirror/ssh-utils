use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use ratatui::{
    backend::Backend,
    buffer::Buffer,
    layout::{Constraint, Layout, Rect},
    style::{Style, Stylize},
    text::{Line, Span, Text},
    widgets::{Paragraph, Widget},
    Frame, Terminal,
};
use std::ops::{Add, Sub};

use crate::{
    config::{
        app_config::{Config, Server},
        app_vault::{self, decrypt_password, encrypt_password, EncryptionKey, Vault},
    },
    helper::convert_to_array,
};

/// current selected item in form
#[derive(Copy, Clone)]
enum CurrentSelect {
    User = 0,
    Ip,
    Port,
    Password,
    Name,
    Shell,
}

/// impl Add and Sub for CurrentSelect
impl Add for CurrentSelect {
    type Output = Self;

    fn add(self, other: Self) -> Self {
        let new_value = (self as isize + other as isize) % 6;
        match new_value {
            0 => CurrentSelect::User,
            1 => CurrentSelect::Ip,
            2 => CurrentSelect::Port,
            3 => CurrentSelect::Password,
            4 => CurrentSelect::Name,
            5 => CurrentSelect::Shell,
            _ => unreachable!(),
        }
    }
}

impl Sub for CurrentSelect {
    type Output = Self;

    fn sub(self, other: Self) -> Self {
        let new_value = (self as isize - other as isize + 6) % 6;
        match new_value {
            0 => CurrentSelect::User,
            1 => CurrentSelect::Ip,
            2 => CurrentSelect::Port,
            3 => CurrentSelect::Password,
            4 => CurrentSelect::Name,
            5 => CurrentSelect::Shell,
            _ => unreachable!(),
        }
    }
}

impl Add<isize> for CurrentSelect {
    type Output = Self;

    fn add(self, other: isize) -> Self {
        let new_value = (self as isize + other).rem_euclid(6);
        match new_value {
            0 => CurrentSelect::User,
            1 => CurrentSelect::Ip,
            2 => CurrentSelect::Port,
            3 => CurrentSelect::Password,
            4 => CurrentSelect::Name,
            5 => CurrentSelect::Shell,
            _ => unreachable!(),
        }
    }
}

impl Sub<isize> for CurrentSelect {
    type Output = Self;

    fn sub(self, other: isize) -> Self {
        let new_value = (self as isize - other).rem_euclid(6);
        match new_value {
            0 => CurrentSelect::User,
            1 => CurrentSelect::Ip,
            2 => CurrentSelect::Port,
            3 => CurrentSelect::Password,
            4 => CurrentSelect::Name,
            5 => CurrentSelect::Shell,
            _ => unreachable!(),
        }
    }
}

/// App holds the state of the application
pub struct ServerCreator<'a> {
    /// Current values of the input boxes
    input: Vec<String>,
    /// Position of cursor in the editor area.
    character_index: usize,
    /// current selected item
    current_select: CurrentSelect,
    /// vault
    vault: &'a mut Vault,
    config: &'a mut Config,
    encryption_key: &'a EncryptionKey,
    mode: CreatorMode,
    server_id: Option<String>,
}

// impl Widget for &mut ServerCreator {
//     fn render(self, area: Rect, buf: &mut Buffer) {
//         let vertical = Layout::vertical([
//             Constraint::Length(1),
//             Constraint::Min(0),
//             Constraint::Length(1)
//         ]);
//         let [head_area, body_area, foot_area] = vertical.areas(area);
//         self.form_position = (body_area.x, body_area.y);
//         self.render_header(head_area, buf);
//         self.render_form(body_area, buf);
//         self.render_footer(foot_area, buf);
//     }
// }
#[derive(PartialEq)]
enum CreatorMode {
    New,
    Edit,
}

impl<'a> ServerCreator<'a> {
    pub fn new(
        vault: &'a mut Vault,
        config: &'a mut Config,
        encryption_key: &'a EncryptionKey,
    ) -> Self {
        Self {
            input: vec![
                String::new(),
                String::new(),
                "22".to_string(),
                String::new(),
                String::new(),
                "bash".to_string(),
            ],
            character_index: 0,
            current_select: CurrentSelect::User,
            vault,
            config,
            encryption_key,
            mode: CreatorMode::New,
            server_id: None,
        }
    }

    pub fn new_edit(
        vault: &'a mut Vault,
        config: &'a mut Config,
        encryption_key: &'a EncryptionKey,
        server_id: &str,
    ) -> Result<Self> {
        let server = config
            .servers
            .iter()
            .find(|s| s.id == server_id)
            .ok_or_else(|| anyhow::anyhow!("can't find server."))?;
        let password = vault
            .servers
            .iter()
            .find(|s| s.id == server_id)
            .ok_or_else(|| anyhow::anyhow!("can't find server password."))?
            .password
            .clone();
        let decrypted_password = decrypt_password(
            &server_id,
            &password,
            &convert_to_array(&encryption_key)
                .map_err(|e| anyhow::anyhow!("encryption key convert failed: {}", e))?,
        )?;
        Ok(Self {
            input: vec![
                server.user.clone(),
                server.ip.clone(),
                server.port.to_string(),
                decrypted_password,
                server.name.clone(),
                server.shell.clone(),
            ],
            character_index: 0,
            current_select: CurrentSelect::User,
            vault,
            config,
            encryption_key,
            mode: CreatorMode::Edit,
            server_id: Some(server_id.to_string()),
        })
    }

    fn render_header(&self, area: Rect, buf: &mut Buffer) {
        let text = Text::from("Enter server details below:").yellow();
        Widget::render(text, area, buf);
    }

    fn render_footer(&self, area: Rect, buf: &mut Buffer) {
        let text = Text::from("  Save (^S), Quit (ESC)").dim();
        Widget::render(text, area, buf);
    }

    fn render_form(&self, area: Rect, buf: &mut Buffer) {
        // highlight currently selected item
        let mut user: Vec<Span> = vec![
            "    user:".into(),
            self.input[CurrentSelect::User as usize].clone().into(),
        ];
        let mut ip: Vec<Span> = vec![
            "      ip:".into(),
            self.input[CurrentSelect::Ip as usize].clone().into(),
        ];
        let mut port: Vec<Span> = vec![
            "    port:".into(),
            self.input[CurrentSelect::Port as usize].clone().into(),
        ];
        // we use * to replace the password
        let password_length = self.input[CurrentSelect::Password as usize].len();
        let masked_password: String = "*".repeat(password_length);
        let mut password: Vec<Span> = vec!["password:".into(), masked_password.into()];
        let mut name: Vec<Span> = vec![
            "    name:".into(),
            self.input[CurrentSelect::Name as usize].clone().into(),
        ];
        let mut shell: Vec<Span> = vec![
            "   shell:".into(),
            self.input[CurrentSelect::Shell as usize].clone().into(),
        ];

        match self.current_select {
            CurrentSelect::User => user[0] = Span::styled("    user:", Style::new().bold()),
            CurrentSelect::Ip => ip[0] = Span::styled("      ip:", Style::new().bold()),
            CurrentSelect::Port => port[0] = Span::styled("    port:", Style::new().bold()),
            CurrentSelect::Password => password[0] = Span::styled("password:", Style::new().bold()),
            CurrentSelect::Name => name[0] = Span::styled("    name:", Style::new().bold()),
            CurrentSelect::Shell => shell[0] = Span::styled("   shell:", Style::new().bold()),
        }

        let user_line = Line::from(user);
        let ip_line = Line::from(ip);
        let port_line = Line::from(port);
        let password_line = if password_length == 0 {
            password[1] =
                Span::styled("leave empty to use the default SSH key", Style::new().dim());
            Line::from(password)
        } else {
            Line::from(password)
        };
        let name_line = Line::from(name);
        let shell_line = Line::from(shell);
        let text = vec![
            user_line,
            ip_line,
            port_line,
            password_line,
            name_line,
            shell_line,
        ];
        let form = Paragraph::new(text);
        Widget::render(&form, area, buf);
    }

    fn move_cursor_left(&mut self) {
        let cursor_moved_left = self.character_index.saturating_sub(1);
        self.character_index = self.clamp_cursor(cursor_moved_left);
    }

    fn move_cursor_right(&mut self) {
        let cursor_moved_right = self.character_index.saturating_add(1);
        self.character_index = self.clamp_cursor(cursor_moved_right);
    }

    fn moveto_current_cursor(&mut self) {
        let cursor_position = self.character_index;
        self.character_index = self.clamp_cursor(cursor_position);
    }

    fn enter_char(&mut self, new_char: char) {
        let index = self.byte_index();
        self.input[self.current_select as usize].insert(index, new_char);
        self.move_cursor_right();
    }

    /// Returns the byte index based on the character position.
    ///
    /// Since each character in a string can be contain multiple bytes, it's necessary to calculate
    /// the byte index based on the index of the character.
    fn byte_index(&mut self) -> usize {
        self.input[self.current_select as usize]
            .char_indices()
            .map(|(i, _)| i)
            .nth(self.character_index)
            .unwrap_or(self.input[self.current_select as usize].len())
    }

    fn delete_char(&mut self) {
        let is_not_cursor_leftmost = self.character_index != 0;
        if is_not_cursor_leftmost {
            // Method "remove" is not used on the saved text for deleting the selected char.
            // Reason: Using remove on String works on bytes instead of the chars.
            // Using remove would require special care because of char boundaries.

            let current_index = self.character_index;
            let from_left_to_current_index = current_index - 1;

            // Getting all characters before the selected character.
            let before_char_to_delete = self.input[self.current_select as usize]
                .chars()
                .take(from_left_to_current_index);
            // Getting all characters after selected character.
            let after_char_to_delete = self.input[self.current_select as usize]
                .chars()
                .skip(current_index);

            // Put all characters together except the selected one.
            // By leaving the selected one out, it is forgotten and therefore deleted.
            self.input[self.current_select as usize] =
                before_char_to_delete.chain(after_char_to_delete).collect();
            self.move_cursor_left();
        }
    }

    fn clamp_cursor(&self, new_cursor_pos: usize) -> usize {
        new_cursor_pos.clamp(0, self.input[self.current_select as usize].chars().count())
    }

    fn move_next_select_item(&mut self) {
        self.current_select = self.current_select + 1;
    }

    fn move_pre_select_item(&mut self) {
        self.current_select = self.current_select - 1;
    }
}

impl<'a> ServerCreator<'a> {
    fn draw(&mut self, terminal: &mut Terminal<impl Backend>) -> Result<()> {
        terminal.draw(|f| ui(f, &self))?;
        Ok(())
    }

    /**
     * Run and get a result
     * true -> add a new server
     * false -> cancelled
     */
    pub fn run(&mut self, mut terminal: &mut Terminal<impl Backend>) -> Result<bool> {
        loop {
            self.draw(&mut terminal)?;
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    match key.code {
                        KeyCode::Char(to_insert) => {
                            // Set this hotkey because of man's habit
                            if to_insert == 'c' {
                                if key.modifiers == event::KeyModifiers::CONTROL {
                                    return Ok(false);
                                }
                            }
                            // Save current server's config
                            if to_insert == 's' {
                                if key.modifiers == event::KeyModifiers::CONTROL {
                                    if self.input.iter().enumerate().any(|(i, input)| {
                                        i != CurrentSelect::Password as usize && input.trim().is_empty()
                                    }) {
                                        continue;
                                    }
                                    let encryption_key = convert_to_array(&self.encryption_key)?;
                                    let mut config_server = Server::new(
                                        self.input[CurrentSelect::Name as usize].clone(),
                                        self.input[CurrentSelect::Ip as usize].clone(),
                                        self.input[CurrentSelect::User as usize].clone(),
                                        self.input[CurrentSelect::Shell as usize].clone(),
                                        self.input[CurrentSelect::Port as usize]
                                            .parse::<u16>()
                                            .unwrap_or(22),
                                    );
                                    if self.mode == CreatorMode::Edit {
                                        let Some(server_id) = self.server_id.clone() else {
                                            return Err(anyhow::anyhow!("Server ID not found"));
                                        };
                                        config_server.id = server_id;
                                    }
                                    let passwd = encrypt_password(
                                        &config_server.id,
                                        self.input[CurrentSelect::Password as usize]
                                            .clone()
                                            .as_str(),
                                        &encryption_key,
                                    )?;
                                    let vault_server =
                                        app_vault::Server::new(config_server.id.clone(), passwd);

                                    if self
                                        .config
                                        .servers
                                        .iter()
                                        .find(|s| s.id == config_server.id)
                                        .is_some()
                                    {
                                        // branch 1: modify server
                                        self.config.modify_server(
                                            config_server.id.as_str(),
                                            config_server.clone(),
                                        )?;
                                        self.vault.modify_server(
                                            config_server.id.as_str(),
                                            vault_server,
                                            &encryption_key,
                                        )?;
                                    } else {
                                        //branch 2: add server
                                        self.config.add_server(config_server.clone())?;
                                        self.vault.add_server(vault_server, &encryption_key)?;
                                    }
                                    return Ok(true);
                                }
                            }
                            self.enter_char(to_insert);
                        }
                        KeyCode::Backspace => {
                            self.delete_char();
                        }
                        KeyCode::Left => {
                            self.move_cursor_left();
                        }
                        KeyCode::Right => {
                            self.move_cursor_right();
                        }
                        KeyCode::Esc => {
                            return Ok(false);
                        }
                        KeyCode::Up => {
                            self.move_pre_select_item();
                            self.moveto_current_cursor();
                        }
                        KeyCode::Down | KeyCode::Enter | KeyCode::Tab => {
                            self.move_next_select_item();
                            self.moveto_current_cursor();
                        }
                        _ => {}
                    }
                }
            }
        }
    }
}

fn ui(f: &mut Frame, server_creator: &ServerCreator) {
    let vertical = Layout::vertical([
        Constraint::Length(1),
        Constraint::Min(0),
        Constraint::Length(1),
    ]);
    let [head_area, body_area, foot_area] = vertical.areas(f.area());
    server_creator.render_header(head_area, f.buffer_mut());
    server_creator.render_form(body_area, f.buffer_mut());
    server_creator.render_footer(foot_area, f.buffer_mut());
    let character_index = server_creator.character_index as u16;
    //due to input character index start at 9
    //eg: "password:"
    //so here add 9
    let cursor_x = body_area.x + character_index + 9;
    let cursor_y = body_area.y + server_creator.current_select as u16;
    f.set_cursor_position((cursor_x, cursor_y));
}