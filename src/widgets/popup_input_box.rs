use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use ratatui::backend::Backend;
use ratatui::style::Style;
use ratatui::text::Line;
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::Terminal;
use anyhow::Result;

use crate::helper;

pub struct PopupInputBox {
    title: String,
    input: String,
}

impl PopupInputBox {
    pub fn new(title: String) -> Self {
        Self {
            title,
            input: String::new(),
        }
    }

    fn render(&self) -> Paragraph {
        let mask_text = "*".repeat(self.input.len());
        let input_text = format!("{}", mask_text);
        let content = vec![Line::from(input_text)];

        Paragraph::new(content)
            .block(
                Block::default()
                    .title(self.title.clone())
                    .borders(Borders::ALL),
            )
            .style(Style::default())
    }

    fn input(&mut self, c: char) {
        self.input.push(c);
    }

    fn backspace(&mut self) {
        self.input.pop();
    }

    fn draw(&self, terminal: &mut Terminal<impl Backend>) -> Result<()> {
        terminal.draw(|f| {
            let area = helper::centered_rect(50, 60, f.area());
            f.render_widget(self.render(), area)
        })?;
        Ok(())
    }

    pub fn run(&mut self, mut terminal: &mut Terminal<impl Backend>) -> Result<Option<String>> {
        loop {
            self.draw(&mut terminal)?;
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    match key.code {
                        KeyCode::Char(to_insert) => {
                            self.input(to_insert);
                        }
                        KeyCode::Backspace => {
                            self.backspace();
                        }
                        KeyCode::Enter => {
                            return Ok(Some(self.input.clone()));
                        }
                        KeyCode::Esc => {
                            return Ok(None);
                        }
                        _ => {}
                    }
                }
            }
        }
    }
}