use crate::audio::{get_devices, start_stream};
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use ratatui::{DefaultTerminal, prelude::*};
use std::{
    io,
    sync::mpsc::{self},
    time::{Duration, Instant},
};

#[derive(Default)]
pub struct App {
    exit: bool,
    audio_stream: Option<cpal::Stream>,
    audio_receiver: Option<mpsc::Receiver<TunerData>>,

    last_tick: Option<Instant>,
    is_referance_pitch_edit_on: bool,
    pub referance_pitch: u16,
    pub referance_pitch_blink_state: bool,

    pub tuner_data: TunerData,

    pub is_popup_open: bool,
    pub devices: Vec<ClatuneDevice>,
    pub selected_device: ClatuneDevice,
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
#[derive(Debug, Default, Clone)]
pub struct ClatuneDevice {
    pub name: String,
    pub id: String,
}

impl App {
    pub fn run(&mut self, terminal: &mut DefaultTerminal) -> io::Result<()> {
        let tick_rate = Duration::from_millis(16); // ~60 Frames Per Second
        self.referance_pitch = 440;
        self.last_tick = Some(Instant::now());
        let (devices, default_device) = get_devices();
        self.devices = devices;
        self.selected_device = default_device;
        self.connect_audio();

        while !self.exit {
            while let Ok(tuner_data) = self.audio_receiver.as_ref().unwrap().try_recv() {
                self.tuner_data = tuner_data;
            }

            terminal.draw(|frame| self.draw(frame))?;
            if event::poll(tick_rate)? {
                self.handle_events()?;
            }
            if self.is_referance_pitch_edit_on {
                self.blink_on_tick();
            }
        }
        Ok(())
    }

    fn connect_audio(&mut self) {
        let (tx, rx) = mpsc::channel::<TunerData>();
        let stream = start_stream(
            tx,
            self.selected_device.id.parse().unwrap(),
            self.referance_pitch,
        );
        self.audio_stream = Some(stream);
        self.audio_receiver = Some(rx);
    }

    fn disconnect_audio(&mut self) {
        self.audio_stream = None;
        self.audio_receiver = None;
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
            KeyCode::Char('i') => {
                self.devices = get_devices().0;
                self.list_selected_index = 0;
                self.is_popup_open = !self.is_popup_open;
            }
            KeyCode::Char('a') => {
                self.is_referance_pitch_edit_on = !self.is_referance_pitch_edit_on;
                self.referance_pitch_blink_state = false;
            }

            _ if self.is_referance_pitch_edit_on => match key_event.code {
                KeyCode::Up | KeyCode::Char('k') | KeyCode::Right | KeyCode::Char('l') => {
                    self.referance_pitch_blink_state = false;
                    self.last_tick = Some(Instant::now());
                    self.referance_pitch = self.referance_pitch + 1;
                }
                KeyCode::Down | KeyCode::Char('j') | KeyCode::Left | KeyCode::Char('h') => {
                    self.referance_pitch_blink_state = false;
                    self.last_tick = Some(Instant::now());
                    self.referance_pitch = self.referance_pitch - 1;
                }
                KeyCode::Esc => self.is_referance_pitch_edit_on = false,
                _ => {}
            },
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
                KeyCode::Enter => {
                    self.selected_device = self.devices[self.list_selected_index].clone();
                    self.disconnect_audio();
                    self.connect_audio();
                    self.is_popup_open = false;
                }
                _ => {}
            },
            _ => {}
        }
    }

    fn blink_on_tick(&mut self) {
        if self.last_tick.unwrap().elapsed() > Duration::from_millis(500) {
            self.referance_pitch_blink_state = !self.referance_pitch_blink_state;
            self.last_tick = Some(Instant::now());
        }
    }

    fn exit(&mut self) {
        self.exit = true;
    }
}
