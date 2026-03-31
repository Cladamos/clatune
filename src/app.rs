use crate::audio::{get_devices, start_stream};
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use ratatui::{DefaultTerminal, prelude::*};
use std::{io, sync::mpsc, time::Duration};

#[derive(Default)]
pub struct App {
    exit: bool,
    _audio_stream: Option<cpal::Stream>,
    pub tuner_data: TunerData,
    pub is_popup_open: bool,
    pub devices: Vec<String>,
    // pub selected_device: usize,
    pub list_selected_index: usize,
}

#[derive(Debug, Default, Clone)]
pub struct AppNote {
    pub note: String,
    pub octave: i32,
    pub is_sharp: bool,
}
#[derive(Debug, Default, Clone)]
pub struct TunerData {
    pub pitches: [AppNote; 3],
    pub cent: i32,
}

impl App {
    /// runs the application's main loop until the user quits
    pub fn run(&mut self, terminal: &mut DefaultTerminal) -> io::Result<()> {
        let tick_rate = Duration::from_millis(16); // ~60 Frames Per Second
        let (tx, rx) = mpsc::channel::<TunerData>();
        let stream = start_stream(tx);
        self._audio_stream = Some(stream);

        if self.is_popup_open || self.devices.is_empty() {
            self.devices = get_devices();
        }

        while !self.exit {
            while let Ok(tuner_data) = rx.try_recv() {
                self.tuner_data = tuner_data;
            }

            terminal.draw(|frame| self.draw(frame))?;
            if event::poll(tick_rate)? {
                self.handle_events()?;
            }
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
            KeyCode::Char('i') => self.is_popup_open = !self.is_popup_open,
            _ if self.is_popup_open => match key_event.code {
                KeyCode::Up | KeyCode::Char('k') => {
                    if self.list_selected_index != 0 {
                        self.list_selected_index = self.list_selected_index - 1;
                    }
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    if self.devices.len() - 1 != self.list_selected_index {
                        self.list_selected_index = self.list_selected_index + 1;
                    }
                }
                KeyCode::Esc => self.is_popup_open = false,
                _ => {}
            },
            _ => {}
        }
    }

    fn exit(&mut self) {
        self.exit = true;
    }
}
