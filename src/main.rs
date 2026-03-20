use std::io;

use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use ratatui::{
    DefaultTerminal, Frame,
    buffer::Buffer,
    layout::{Alignment, Constraint, Layout, Margin, Rect},
    style::Stylize,
    symbols::border,
    text::Line,
    widgets::{Block, Paragraph, Widget},
};

fn main() -> io::Result<()> {
    ratatui::run(|terminal| App::default().run(terminal))
}

#[derive(Debug, Default)]
pub struct App {
    exit: bool,
}

impl App {
    /// runs the application's main loop until the user quits
    pub fn run(&mut self, terminal: &mut DefaultTerminal) -> io::Result<()> {
        while !self.exit {
            terminal.draw(|frame| self.draw(frame))?;
            self.handle_events()?;
        }
        Ok(())
    }

    fn draw(&self, frame: &mut Frame) {
        frame.render_widget(self, frame.area());
    }

    /// updates the application's state based on user input
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
        match key_event.code {
            KeyCode::Char('q') => self.exit(),
            KeyCode::Char('c') if key_event.modifiers.contains(KeyModifiers::CONTROL) => {
                self.exit()
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
        let title = Line::from(" Clatune ".bold());
        let instructions = Line::from(vec![" Quit ".into(), "<Q> ".blue().bold()]);
        let main_block = Block::bordered()
            .title(title.centered())
            .title_bottom(instructions.centered())
            .border_set(border::ROUNDED);
        main_block.render(area, buf);

        fn centered_rect(width: u16, height: u16, r: Rect) -> Rect {
            let padding_vertical = r.height.saturating_sub(height) / 2;
            let padding_horizontal = r.width.saturating_sub(width) / 2;

            Rect {
                x: r.x + padding_horizontal,
                y: r.y + padding_vertical,
                width: width.min(r.width),
                height: height.min(r.height),
            }
            .inner(Margin {
                horizontal: 2,
                vertical: 0,
            })
        }

        let tuner_area = centered_rect(70, 5, area);
        let tuner_layout =
            Layout::vertical([Constraint::Min(1), Constraint::Min(3), Constraint::Min(1)])
                .split(tuner_area);
        let up_arrow = Paragraph::new("▲").alignment(Alignment::Center);
        let down_arrow = Paragraph::new("▼").alignment(Alignment::Center);
        let tuner_block = Block::bordered().border_set(border::ROUNDED);

        down_arrow.render(tuner_layout[0], buf);
        tuner_block.render(tuner_layout[1], buf);
        up_arrow.render(tuner_layout[2], buf);
    }
}
