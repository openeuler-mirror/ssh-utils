use std::ops::{Add, Sub};

use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use ratatui::{backend::Backend, 
    buffer::Buffer,
    layout::{Constraint, Layout, Rect},
    style::{Style, Stylize},
    text::{Line, Span, Text},
    widgets::{Paragraph, Widget},
    Terminal};
use anyhow::Result;

/// current selected item in form
#[derive(Copy, Clone)]
enum CurrentSelect {
    User = 0,
    Ip,
    Password,
    Name,
}

/// impl Add and Sub for CurrentSelect
impl Add for CurrentSelect {
    type Output = Self;

    fn add(self, other: Self) -> Self {
        let new_value = (self as isize + other as isize) % 4;
        match new_value {
            0 => CurrentSelect::User,
            1 => CurrentSelect::Ip,
            2 => CurrentSelect::Password,
            3 => CurrentSelect::Name,
            _ => unreachable!(),
        }
    }
}

impl Sub for CurrentSelect {
    type Output = Self;

    fn sub(self, other: Self) -> Self {
        let new_value = (self as isize - other as isize + 4) % 4;
        match new_value {
            0 => CurrentSelect::User,
            1 => CurrentSelect::Ip,
            2 => CurrentSelect::Password,
            3 => CurrentSelect::Name,
            _ => unreachable!(),
        }
    }
}

impl Add<isize> for CurrentSelect {
    type Output = Self;

    fn add(self, other: isize) -> Self {
        let new_value = (self as isize + other).rem_euclid(4);
        match new_value {
            0 => CurrentSelect::User,
            1 => CurrentSelect::Ip,
            2 => CurrentSelect::Password,
            3 => CurrentSelect::Name,
            _ => unreachable!(),
        }
    }
}

impl Sub<isize> for CurrentSelect {
    type Output = Self;

    fn sub(self, other: isize) -> Self {
        let new_value = (self as isize - other).rem_euclid(4);
        match new_value {
            0 => CurrentSelect::User,
            1 => CurrentSelect::Ip,
            2 => CurrentSelect::Password,
            3 => CurrentSelect::Name,
            _ => unreachable!(),
        }
    }
}

/// App holds the state of the application
pub struct ServerCreator {
    /// Current values of the input boxes
    input: Vec<String>,
    /// Position of cursor in the editor area.
    character_index: usize,
    /// current selected item
    current_select: CurrentSelect,
    /// form position
    /// used to set cursor
    form_position: (u16, u16)
}

impl Widget for &mut ServerCreator {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let vertical = Layout::vertical([
            Constraint::Length(1),
            Constraint::Min(0),
            Constraint::Length(1)
        ]);
        let [head_area, body_area, foot_area] = vertical.areas(area);
        self.form_position = (body_area.x, body_area.y);
        self.render_header(head_area, buf);
        self.render_form(body_area, buf);
        self.render_footer(foot_area, buf);
    }
}

impl ServerCreator {
    pub fn new() -> Self {
        Self {
            input: vec![String::new(), String::new(), String::new(), String::new()],
            character_index: 0,
            current_select: CurrentSelect::User,
            form_position: (0,3)

        }
    }

    fn render_header(&self, area: Rect, buf: &mut Buffer) {
        let text = Text::from("Enter server information below:").yellow();
        Widget::render(text, area, buf);
    }

    fn render_footer(&self, area: Rect, buf: &mut Buffer) {
        let text = Text::from("  Save (^S), Quit (ESC)").dim();
        Widget::render(text, area, buf);
    }

    fn render_form(&self, area: Rect, buf: &mut Buffer) {
        // highlight currently selected item
        let mut user: Vec<Span> =     vec!["    user:".into(), self.input[CurrentSelect::User as usize].clone().into()];
        let mut ip: Vec<Span> =       vec!["      ip:".into(), self.input[CurrentSelect::Ip as usize].clone().into()];
        let mut password: Vec<Span> = vec!["password:".into(), self.input[CurrentSelect::Password as usize].clone().into()];
        let mut name: Vec<Span> =     vec!["    name:".into(), self.input[CurrentSelect::Name as usize].clone().into()];
        
        match self.current_select {
            CurrentSelect::User => user[0] = Span::styled("    user:", Style::new().bold()),
            CurrentSelect::Ip => ip[0] = Span::styled("      ip:", Style::new().bold()),
            CurrentSelect::Password => password[0] = Span::styled("password:", Style::new().bold()),
            CurrentSelect::Name => name[0] = Span::styled("    name:", Style::new().bold()),
        }
        
        let user_line = Line::from(user);
        let ip_line = Line::from(ip);
        let password_line = Line::from(password);
        let name_line = Line::from(name);
        let text = vec![user_line, ip_line, password_line, name_line];
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
            let before_char_to_delete = self.input[self.current_select as usize].chars().take(from_left_to_current_index);
            // Getting all characters after selected character.
            let after_char_to_delete = self.input[self.current_select as usize].chars().skip(current_index);

            // Put all characters together except the selected one.
            // By leaving the selected one out, it is forgotten and therefore deleted.
            self.input[self.current_select as usize] = before_char_to_delete.chain(after_char_to_delete).collect();
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


impl ServerCreator {
    fn draw(&mut self, terminal: &mut Terminal<impl Backend>) -> Result<()> {
        terminal.draw(|f| {
            let character_index = self.character_index as u16;
            let form_position = self.form_position;
            let cursor_x = form_position.0 + character_index + 9;
            let cursor_y = form_position.1 + self.current_select as u16;

            f.render_widget(self, f.size());
            f.set_cursor(cursor_x,cursor_y);
        })?;
        Ok(())
    }

    pub fn run(&mut self, mut terminal: &mut Terminal<impl Backend>) -> Result<()> {
        loop{
            self.draw(&mut terminal)?;
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    match key.code {
                        KeyCode::Char(to_insert) => {
                            // Set this hotkey because of man's habit
                            if to_insert == 'c' {
                                if key.modifiers == event::KeyModifiers::CONTROL {
                                    return Ok(())
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
                            return Ok(());
                        }
                        KeyCode::Up => {
                            self.move_pre_select_item();
                            self.moveto_current_cursor();
                        }
                        KeyCode::Down | KeyCode::Enter => {
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

#[test]
fn run_widget() -> Result<()> {
    crossterm::terminal::enable_raw_mode()?;
    let stdout = std::io::stdout();
    crossterm::execute!(std::io::stdout(), crossterm::terminal::EnterAlternateScreen)?;
    let backend = ratatui::backend::CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    let mut app = ServerCreator::new();

    app.run(&mut terminal)?;
    crossterm::execute!(std::io::stdout(), crossterm::terminal::LeaveAlternateScreen)?;
    crossterm::terminal::disable_raw_mode()?;
    Ok(())
}