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
            if self.timer >= 30 {
                self.timer = 30;
                self.app_state = AppState::Ended;
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
            (KeyCode::Char('r'), KeyModifiers::CONTROL) => self.restart(),
            (KeyCode::Char(c), KeyModifiers::NONE | KeyModifiers::SHIFT) => {
                if self.cursor_index == self.task_string.len() {
                    self.app_state = AppState::Ended;
                }

                if self.app_state == AppState::NotTyping {
                    self.start_time = SystemTime::now();
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

    fn restart(&mut self) {
        self.app_state = AppState::NotTyping;
        self.cursor_index = 0;
        self.task_string = get_task_string();
        self.user_string = String::new();
        self.words_typed = 0;
        self.error_count = 0;
        self.timer = 0;
    }
}

impl Widget for &App {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let max_width = 62;
        let centered_width = area.width.min(max_width);
        let x_offset = (area.width.saturating_sub(centered_width)) / 2;

        let centered_area = Rect {
            x: area.x + x_offset,
            y: area.y,
            width: centered_width,
            height: area.height,
        };

        let title = Line::from(" ████████╗██╗   ██╗██████╗ ███████╗██████╗  ".bold());
        let title2 = Line::from(" ╚══██╔══╝╚██╗ ██╔╝██╔══██╗██╔════╝██╔══██╗ ".bold());
        let title3 = Line::from("    ██║    ╚████╔╝ ██████╔╝█████╗  ██████╔╝ ".bold());
        let title4 = Line::from("    ██║     ╚██╔╝  ██╔═══╝ ██╔══╝  ██╔══██╗ ".bold());
        let title5 = Line::from("    ██║      ██║   ██║     ███████╗██║  ██║ ".bold());
        let title6 = Line::from("    ╚═╝      ╚═╝   ╚═╝     ╚══════╝╚═╝  ╚═╝ ".bold());

        let instructions = Line::from(vec![" [Esc] Quit ".into(), " [Ctrl+R] Restart ".into()]);

        let block = Block::bordered()
            .title_bottom(instructions.centered())
            .border_set(border::THICK);

        let inner_area = block.inner(centered_area);
        block.render(area, buf);

        let title_area = Rect {
            x: inner_area.x,
            y: inner_area.y,
            width: inner_area.width,
            height: 6,
        };

        let title_lines = vec![title, title2, title3, title4, title5, title6];
        Paragraph::new(title_lines)
            .centered()
            .render(title_area, buf);

        let content_area = Rect {
            x: inner_area.x,
            y: inner_area.y + 7,
            width: inner_area.width,
            height: inner_area.height.saturating_sub(7),
        };

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

        let minutes = self.timer as f64 / 60.0;
        let wpm = if minutes > 0.0 && self.app_state != AppState::NotTyping {
            (self.words_typed as f64 / minutes) as u32
        } else {
            0
        };

        let accuracy = if self.cursor_index > 0 {
            let correct_chars = self.cursor_index.saturating_sub(self.error_count as usize);
            ((correct_chars as f64 / self.cursor_index as f64) * 100.0) as u32
        } else {
            100
        };

        let timer_display = format!("{:02}/30", self.timer);

        let stats_line = Line::from(vec![
            Span::from("  ⏱  "),
            Span::from(timer_display).yellow().bold(),
            Span::from("     ⚡ "),
            Span::from(wpm.to_string()).cyan().bold(),
            Span::from(" WPM     ✓ "),
            Span::from(format!("{}%", accuracy)).green().bold(),
        ]);

        let mut display_lines = vec![
            Line::from(""),
            Line::from(task_string_styled),
            Line::from(""),
            Line::from("  ─".repeat(20)), // separator
            stats_line,
            Line::from(""),
        ];

        if self.app_state == AppState::Ended {
            let final_stats = Line::from(vec![
                Span::from("  Final Results - "),
                Span::from("Errors: "),
                Span::from(self.error_count.to_string()).red().bold(),
                Span::from("  Words: "),
                Span::from(self.words_typed.to_string()).green().bold(),
            ]);
            display_lines.push(final_stats);
        }

        Paragraph::new(display_lines)
            .wrap(Wrap { trim: true })
            .render(content_area, buf);
    }
}
