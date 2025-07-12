use std::io;

use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use ratatui::{
    DefaultTerminal, Frame,
    buffer::Buffer,
    layout::Rect,
    style::Stylize,
    symbols::border,
    text::{Line, Text},
    widgets::{Block, Paragraph, Widget},
};

fn main() -> io::Result<()> {
    let mut terminal = ratatui::init(); // initilize a terminal
    let app_result = App::new().run(&mut terminal); // pass the terminal in the run
    ratatui::restore(); // when exited, restore back previous sessions
    app_result // return back the error to user if any
}

// out app's state will be stored in this
pub struct App {
    task_string: String,
    user_string: String,
    state: State,
    cursor_index: usize,
    wpm: u8,
    error_count: u8,
    time_remaining: u8,
    exit: bool,
}

pub enum State {
    NotTyping,
    Typing,
    Paused,
    Ended,
}

impl App {
    pub fn new() -> Self {
        Self {
            task_string: String::new(),
            user_string: String::new(),
            state: State::NotTyping,
            cursor_index: 0,
            wpm: 0,
            error_count: 0,
            time_remaining: 30,
            exit: false,
        }
    }

    /// runs the application's main loop until the user quits
    pub fn run(&mut self, terminal: &mut DefaultTerminal) -> io::Result<()> {
        while !self.exit {
            // takens an closure, or annonomus function to draw the results
            terminal.draw(|frame| self.draw(frame))?;
            // handle the key events
            self.handle_events()?;
        }
        Ok(())
    }

    fn draw(&self, frame: &mut Frame) {
        frame.render_widget(self, frame.area());
    }

    fn handle_events(&mut self) -> io::Result<()> {
        match event::read()? {
            // it's important to check that the event is a key press event as
            // crossterm also emits key release and repeat events on Windows.
            Event::Key(key_event) if key_event.kind == KeyEventKind::Press => {
                self.handle_key_event(key_event)
            }
            _ => {}
        };
        Ok(())
    }

    fn handle_key_event(&mut self, key_event: KeyEvent) {
        match (key_event.code, key_event.modifiers) {
            (KeyCode::Char('c'), KeyModifiers::CONTROL) => self.exit(),
            (KeyCode::Esc, KeyModifiers::NONE) => self.exit(),
            _ => {
            }
        }
    }

    fn exit(&mut self) {
        self.exit = true;
    }
}

impl Widget for &App {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let title = Line::from(" TYPER TUI ".bold());

        let instructions = Line::from(vec![" Quit Esc or CLTR + ".into(), "<C> ".blue().bold()]);

        let block = Block::bordered()
            .title(title.centered())
            .title_bottom(instructions.centered())
            .border_set(border::THICK);

        let task_string = Text::from(vec![Line::from(vec![
            self.user_string.to_string().yellow(),
        ])]);

        Paragraph::new(task_string)
            .centered()
            .block(block)
            .render(area, buf);
    }
}
