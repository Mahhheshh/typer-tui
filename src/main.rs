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
    sentences: Vec<String>,
    current_sentence_index: usize,
    sentence_cursor_index: usize,
}

#[derive(PartialEq, Eq)]
pub enum AppState {
    NotTyping,
    Typing,
    Paused,
    Ended,
}

fn get_task_string() -> (String, Vec<String>) {
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

    let mut result = String::new();
    for _ in 0..255 {
        let random_index = rand::random_range(0..word_vec.len());
        let word = word_vec[random_index].trim();
        if word.len() == 0 {
            continue;
        }
        result.push_str(word);
        result.push(' ');
    }

    // Split into sentences
    let words: Vec<&str> = result.split_whitespace().collect();
    let mut sentences = Vec::new();
    let mut current_sentence = String::new();
    let mut word_count = 0;

    for word in words {
        if !current_sentence.is_empty() {
            current_sentence.push(' ');
        }
        current_sentence.push_str(word);
        word_count += 1;

        // Create a sentence every 10-15 words
        if word_count >= 10 + (rand::random_range(0..6)) {
            sentences.push(current_sentence.clone());
            current_sentence.clear();
            word_count = 0;
        }
    }

    if !current_sentence.is_empty() {
        sentences.push(current_sentence);
    }

    (result, sentences)
}

impl App {
    pub fn new() -> Self {
        let (task_string, sentences) = get_task_string();
        Self {
            task_string,
            user_string: String::new(),
            app_state: AppState::NotTyping,
            cursor_index: 0,
            words_typed: 0,
            error_count: 0,
            timer: 0,
            start_time: SystemTime::now(),
            exit: false,
            sentences,
            current_sentence_index: 0,
            sentence_cursor_index: 0,
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
                if self.current_sentence_index >= self.sentences.len() {
                    self.app_state = AppState::Ended;
                    return;
                }

                if self.app_state == AppState::NotTyping {
                    self.start_time = SystemTime::now();
                    self.app_state = AppState::Typing;
                }

                let expected_char = self.get_current_sentence_char().unwrap_or_default();

                if expected_char != c && self.app_state == AppState::Typing {
                    self.error_count += 1;
                    if expected_char != ' ' && c == ' ' {
                        // skipping the word logic
                        let current_sentence = &self.sentences[self.current_sentence_index];
                        let remaining_chars = &current_sentence[self.sentence_cursor_index..];
                        if let Some(space_pos) = remaining_chars.find(' ') {
                            let spaces_to_add = space_pos;
                            self.user_string.push_str(&" ".repeat(spaces_to_add));
                            self.sentence_cursor_index += spaces_to_add;
                            self.cursor_index += spaces_to_add;
                        }
                    }
                }

                if self.app_state == AppState::Typing {
                    if expected_char == c && expected_char == ' ' {
                        self.words_typed += 1;
                    }
                    self.user_string.push(c);
                    self.sentence_cursor_index += 1;
                    self.cursor_index += 1;

                    // Check if current sentence is complete
                    if self.is_current_sentence_complete() {
                        self.advance_to_next_sentence();
                    }
                }
            }

            (KeyCode::Backspace, KeyModifiers::NONE) => {
                if !self.user_string.is_empty()
                    && self.cursor_index != 0
                    && self.app_state == AppState::Typing
                {
                    self.user_string.pop();
                    self.cursor_index -= 1;
                    if self.sentence_cursor_index > 0 {
                        self.sentence_cursor_index -= 1;
                    }
                }
            }
            _ => {}
        }
    }

    fn exit(&mut self) {
        self.exit = true;
    }

    fn restart(&mut self) {
        let (task_string, sentences) = get_task_string();
        self.app_state = AppState::NotTyping;
        self.cursor_index = 0;
        self.task_string = task_string;
        self.sentences = sentences;
        self.user_string = String::new();
        self.words_typed = 0;
        self.error_count = 0;
        self.timer = 0;
        self.current_sentence_index = 0;
        self.sentence_cursor_index = 0;
    }

    fn get_visible_sentences(&self) -> String {
        let start_idx = self.current_sentence_index;
        let end_idx = (start_idx + 3).min(self.sentences.len());

        if start_idx >= self.sentences.len() {
            return String::new();
        }

        self.sentences[start_idx..end_idx].join(" ")
    }

    fn get_current_sentence_char(&self) -> Option<char> {
        if self.current_sentence_index >= self.sentences.len() {
            return None;
        }

        let current_sentence = &self.sentences[self.current_sentence_index];
        current_sentence.chars().nth(self.sentence_cursor_index)
    }

    fn advance_to_next_sentence(&mut self) {
        self.current_sentence_index += 1;
        self.sentence_cursor_index = 0;

        self.user_string.clear();
        self.cursor_index = 0;
    }

    fn is_current_sentence_complete(&self) -> bool {
        if self.current_sentence_index >= self.sentences.len() {
            return true;
        }

        let current_sentence = &self.sentences[self.current_sentence_index];
        self.sentence_cursor_index >= current_sentence.len()
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
        let visible_text = self.get_visible_sentences();

        for (i, task_char) in visible_text.chars().enumerate() {
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
        let sentence_progress = format!("Line {}/{}",
            self.current_sentence_index + 1,
            self.sentences.len().min(self.current_sentence_index + 3));

        let stats_line = Line::from(vec![
            Span::from("  ⏱  "),
            Span::from(timer_display).yellow().bold(),
            Span::from("     ⚡ "),
            Span::from(wpm.to_string()).cyan().bold(),
            Span::from(" WPM     ✓ "),
            Span::from(format!("{}%", accuracy)).green().bold(),
            Span::from("     📄 "),
            Span::from(sentence_progress).blue().bold(),
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
