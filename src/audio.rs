use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use pitch_calc::{Hz, Letter, letter_octave_from_hz, step_from_letter_octave};
use pitch_detection::detector::PitchDetector;
use pitch_detection::detector::mcleod::McLeodDetector;
use std::sync::mpsc::Sender;

use crate::app::{AppNote, TunerData};

pub fn get_devices() -> (Vec<String>, String) {
    let host = cpal::default_host();
    let devices = host
        .devices()
        .unwrap()
        .filter(|device| device.supports_input())
        .map(|device| device.description().unwrap().name().to_string())
        .collect();
    let default_device = host
        .default_input_device()
        .unwrap()
        .description()
        .unwrap()
        .name()
        .to_string();
    (devices, default_device)
}

pub fn start_stream(tx: Sender<TunerData>, device_name: String) -> cpal::Stream {
    let host = cpal::default_host();
    let device = host
        .devices()
        .unwrap()
        .find(|device| device.description().unwrap().name() == device_name)
        .expect("No default input device found");
    let mut supported_configs_range = device
        .supported_input_configs()
        .expect("error while querying configs");
    let supported_config = supported_configs_range
        .next()
        .expect("no supported config?!")
        .with_max_sample_rate();

    let sample_rate = supported_config.sample_rate() as usize;
    let mut input_buffer: Vec<f32> = Vec::with_capacity(4096);

    let stream = device
        .build_input_stream(
            &supported_config.config(),
            move |data: &[f32], _| {
                input_buffer.extend_from_slice(data);
                while input_buffer.len() >= 4096 {
                    let signal = &input_buffer[0..4096];
                    let mut detector = McLeodDetector::new(4096, 2048);
                    if let Some(pitch) = detector.get_pitch(&signal, sample_rate, 0.008, 0.6) {
                        let _ = tx.send(get_tuner_data(pitch.frequency));
                    }
                    input_buffer.drain(0..2048);
                }
            },
            |err| eprintln!("Error: {:?}", err),
            None,
        )
        .expect("No default input stream found");

    stream.play().expect("Failed to start stream");
    stream
}

fn letter_to_string(letter: Letter) -> &'static str {
    match letter {
        Letter::C => "C",
        Letter::Csh => "C#",
        Letter::Db => "C#",
        Letter::D => "D",
        Letter::Dsh => "D#",
        Letter::Eb => "D#",
        Letter::E => "E",
        Letter::F => "F",
        Letter::Fsh => "F#",
        Letter::Gb => "F#",
        Letter::G => "G",
        Letter::Gsh => "G#",
        Letter::Ab => "G#",
        Letter::A => "A",
        Letter::Ash => "A#",
        Letter::Bb => "A#",
        Letter::B => "B",
    }
}

fn get_tuner_data(frequency: f32) -> TunerData {
    const NOTES: [&str; 12] = [
        "C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B",
    ];
    let (note, octave) = letter_octave_from_hz(frequency);

    let input_step = Hz(frequency).to_step().0;
    let nearest_step = step_from_letter_octave(note, octave);

    // I guess in this crate stesp are semitones
    // When I am trying calculate with steps it showed 2x more than it should
    // In normal 1 step = 2 semitones = 200 cents
    let cent_diff = ((input_step - nearest_step) * 100.0).round() as i32;

    let index = NOTES
        .iter()
        .position(|&r| r == letter_to_string(note))
        .unwrap();
    let mut indexes = [0, index, index + 1];
    let mut octaves = [octave, octave, octave];
    if index == 0 {
        indexes[0] = 11;
        octaves[0] = octave - 1;
    } else {
        indexes[0] = index - 1;
    }
    if index == 11 {
        indexes[2] = 0;
        octaves[2] = octave + 1;
    }

    let notes: [AppNote; 3] = std::array::from_fn(|i| AppNote {
        note: NOTES[indexes[i]].to_string(),
        octave: octaves[i],
        is_sharp: NOTES[indexes[i]].contains("#"),
    });

    TunerData {
        pitches: notes,
        cent: cent_diff,
    }
}
