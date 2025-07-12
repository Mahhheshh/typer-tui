use rand::seq::SliceRandom;
use std::{
    fs::File,
    io::{self, Read},
    path::Path,
    time::{Duration, SystemTime},
};

use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use ratatui::{
    DefaultTerminal, Frame,
    buffer::Buffer,
    layout::Rect,
    style::Stylize,
    symbols::border,
    text::{Line, Span},
    widgets::{Block, Paragraph, Widget, Wrap},
};

fn main() -> io::Result<()> {
    let mut terminal = ratatui::init();
    let app_result = App::new().run(&mut terminal);
    ratatui::restore();
    app_result
}

// our app's state will be stored in this
pub struct App {
    task_string: String,
    user_string: String,
    app_state: AppState,
    cursor_index: usize,
    words_typed: u8,
    error_count: u8,
    start_time: SystemTime,
    timer: u64,
    exit: bool,
}

#[derive(PartialEq, Eq)]
pub enum AppState {
    NotTyping,
    Typing,
    Paused,
    Ended,
}

fn get_task_string() -> String {
    let file_path = Path::new("src/word.txt");

    let mut file_object = match File::open(&file_path) {
        Ok(file) => file,
        Err(error) => panic!("couldn't open {}", error),
    };

    let mut s = String::new();

    match file_object.read_to_string(&mut s) {
        Err(error) => panic!("couldn't read {}", error),
        Ok(_) => {}
    }

    let word_vec: Vec<&str> = s.split(' ').collect();
    let mut rng = rand::rng();
    let mut shuffled_words = word_vec.clone();
    shuffled_words.shuffle(&mut rng);

    // println!("word vec length: {:?}", word_vec.len());

    let mut result = String::new();
    for _ in 0..255 {
        // no one is using this typer, and will have more than 300 words per minute speed
        let random_index = rand::random_range(0..word_vec.len());
        let word = word_vec[random_index].trim();
        if word.len() == 0 {
            continue;
        }
        result.push_str(word);
        result.push(' ');
    }

    result
}

impl App {
    pub fn new() -> Self {
        Self {
            task_string: get_task_string(),
            user_string: String::new(),
            app_state: AppState::NotTyping,
            cursor_index: 0,
            words_typed: 0,
            error_count: 0,
            timer: 0,
            start_time: SystemTime::now(),
            exit: false,
        }
    }

    pub fn run(&mut self, terminal: &mut DefaultTerminal) -> io::Result<()> {
        while !self.exit {
            terminal.draw(|frame| self.draw(frame))?;
            self.handle_events()?;
            if self.app_state == AppState::Typing {
                self.timer = self.start_time.elapsed().unwrap().as_secs();
            }
        }
        Ok(())
    }

    fn draw(&self, frame: &mut Frame) {
        frame.render_widget(self, frame.area());
    }

    fn handle_events(&mut self) -> io::Result<()> {
        if event::poll(Duration::from_millis(100))? {
            match event::read()? {
                Event::Key(key_event) if key_event.kind == KeyEventKind::Press => {
                    self.handle_key_event(key_event)
                }
                _ => {}
            }
        }
        Ok(())
    }

    fn handle_key_event(&mut self, key_event: KeyEvent) {
        match (key_event.code, key_event.modifiers) {
            (KeyCode::Char('c'), KeyModifiers::CONTROL) => self.exit(),
            (KeyCode::Esc, KeyModifiers::NONE) => self.exit(),

            (KeyCode::Char(c), KeyModifiers::NONE | KeyModifiers::SHIFT) => {
                if self.cursor_index == self.task_string.len() {
                    self.app_state = AppState::Ended;
                }

                if self.app_state == AppState::NotTyping {
                    self.app_state = AppState::Typing;
                }

                let expected_char = self
                    .task_string
                    .chars()
                    .nth(self.cursor_index)
                    .unwrap_or_default();

                if expected_char != c && self.app_state == AppState::Typing {
                    self.error_count += 1;
                    if expected_char != ' ' && c == ' ' {
                        // skipping the word logic
                        let slice = self
                            .task_string
                            .get(self.cursor_index..)
                            .and_then(|s| s.find(' '));

                        let mut spaces_to_add =
                            self.task_string.len().saturating_sub(self.cursor_index);
                        if let Some(spaces) = slice {
                            spaces_to_add = spaces;
                        }
                        self.user_string.push_str(&" ".repeat(spaces_to_add));
                        self.cursor_index += spaces_to_add;
                    }
                }

                if self.app_state == AppState::Typing {
                    if expected_char == c && expected_char == ' ' {
                        self.words_typed += 1;
                    }
                    self.user_string.push(c);
                    self.cursor_index += 1;
                }
            }

            (KeyCode::Backspace, KeyModifiers::NONE) => {
                if !self.user_string.is_empty()
                    && self.cursor_index != 0
                    && self.app_state == AppState::Typing
                {
                    self.user_string.pop();
                    self.cursor_index -= 1;
                }
            }
            _ => {}
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

        let inner_area = block.inner(area);
        block.render(area, buf);

        let mut task_string_styled = Vec::new();
        for (i, task_char) in self.task_string.chars().enumerate() {
            let styled_char = if i < self.user_string.len() {
                let user_char = self.user_string.chars().nth(i).unwrap();
                if task_char == user_char {
                    task_char.to_string().green()
                } else {
                    task_char.to_string().red().underlined()
                }
            } else if i == self.user_string.len() {
                task_char.to_string().black().on_white()
            } else {
                task_char.to_string().dim()
            };
            task_string_styled.push(styled_char);
        }

        let task_string = Line::from(task_string_styled);
        let timer_line = Line::from(vec![
            Span::from("Time: "),
            Span::from(self.timer.to_string()).yellow().bold(),
        ]);

        let mut display_lines = vec![task_string, timer_line];
        if self.app_state == AppState::Ended {
            let error_line = Line::from(vec![
                Span::from("Errors: "),
                Span::from(self.error_count.to_string()).red().bold(),
            ]);
            let words_typed_line = Line::from(vec![
                Span::from("Words typed: "),
                Span::from(self.words_typed.to_string()).green().bold(),
            ]);
            display_lines.push(error_line);
            display_lines.push(words_typed_line);

            if self.timer > 0 {
                let wpm = (self.words_typed as f64 / self.timer as f64) * 60.0;
                let wpm_line = Line::from(vec![
                    Span::from("WPM: "),
                    Span::from(format!("{:.1}", wpm)).cyan().bold(),
                ]);
                display_lines.push(wpm_line);
            }
        }

        Paragraph::new(display_lines)
            .wrap(Wrap { trim: true })
            .centered()
            .render(inner_area, buf);
    }
}
