use cpal::{
    DeviceId,
    traits::{DeviceTrait, HostTrait, StreamTrait},
};
use pitch_calc::{Hz, Letter, letter_octave_from_hz, step_from_letter_octave};
use pitch_detection::detector::PitchDetector;
use pitch_detection::detector::mcleod::McLeodDetector;
use std::sync::mpsc::Sender;

use crate::app::{AppNote, ClatuneDevice, TunerData};

fn get_best_host() -> cpal::Host {
    let available_hosts = cpal::available_hosts();

    if available_hosts.contains(&cpal::HostId::PipeWire) {
        return cpal::host_from_id(cpal::HostId::PipeWire).expect("Failed to initialize PipeWire");
    }

    if available_hosts.contains(&cpal::HostId::PulseAudio) {
        return cpal::host_from_id(cpal::HostId::PulseAudio)
            .expect("Failed to initialize PulseAudio");
    }

    cpal::default_host()
}

pub fn get_devices() -> (Vec<ClatuneDevice>, ClatuneDevice) {
    let host = get_best_host();
    let devices: Vec<ClatuneDevice> = host
        .input_devices()
        .expect("No devices found")
        .map(|device| ClatuneDevice {
            name: device.description().unwrap().name().to_string(),
            id: device.id().unwrap().to_string(),
        })
        .collect();
    let default_device = host
        .default_input_device()
        .expect("Default device couldnt found");

    (
        devices,
        ClatuneDevice {
            name: default_device.description().unwrap().name().to_string(),
            id: default_device.id().unwrap().to_string(),
        },
    )
}

pub fn start_stream(
    tx: Sender<TunerData>,
    device_id: DeviceId,
    referance_pitch: u16,
) -> cpal::Stream {
    let host = get_best_host();
    let device = host
        .input_devices()
        .expect("No devices found")
        .find(|device| device.id().unwrap() == device_id)
        .expect("Selected input device cannot found");
    let mut supported_configs_range = device
        .supported_input_configs()
        .expect("Error while querying configs");
    let supported_config = supported_configs_range
        .next()
        .expect("No supported config")
        .with_max_sample_rate();

    let sample_rate = supported_config.sample_rate() as usize;
    let mut input_buffer: Vec<f32> = Vec::with_capacity(8192);

    let stream = device
        .build_input_stream(
            supported_config.config(),
            move |data: &[f32], _| {
                input_buffer.extend_from_slice(data);
                let mut smoothed_freq = 0.0;
                let mut detector = McLeodDetector::new(4096, 2048);
                let mut lpf_state = 0.0;

                while input_buffer.len() >= 4096 {
                    // RMS Noise Gate
                    let rms = (input_buffer.iter().map(|&x| x * x).sum::<f32>()
                        / input_buffer.len() as f32)
                        .sqrt();
                    if rms > 0.001 {
                        let signal = &input_buffer[0..4096];

                        // Low Pass Filter
                        // Alpha is between 0 and 1 (0 = no output, 1 = no filtering)
                        let lpf_alpha = 0.15;
                        let filtered_signal: Vec<f32> = signal
                            .iter()
                            .map(|&x| {
                                // First-Order IIR Filter Math
                                lpf_state = lpf_state + lpf_alpha * (x - lpf_state);
                                lpf_state
                            })
                            .collect();

                        if let Some(pitch) =
                            detector.get_pitch(&filtered_signal, sample_rate, 0.008, 0.6)
                        {
                            // Smooth the signal
                            // It smoothes the rapid frequency jumps with using previous values
                            // Alpha is between 0 and 1 (0 = no smoothing, 1 = no new data)
                            if pitch.frequency > 20.0 && pitch.frequency < 2000.0 {
                                let s_alpha = 0.3;
                                if smoothed_freq == 0.0 {
                                    smoothed_freq = pitch.frequency;
                                } else {
                                    smoothed_freq = (s_alpha * pitch.frequency)
                                        + ((1.0 - s_alpha) * smoothed_freq);
                                }

                                let _ = tx.send(get_tuner_data(smoothed_freq, referance_pitch));
                            }
                        }
                    } else {
                        smoothed_freq = 0.0;
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

fn get_tuner_data(frequency: f32, referance_pitch: u16) -> TunerData {
    const NOTES: [&str; 12] = [
        "C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B",
    ];

    // I'm not tested that yet
    let hz_diff = (referance_pitch - 440) as f32;
    let (note, octave) = letter_octave_from_hz(frequency - hz_diff);

    let input_step = Hz(frequency - hz_diff).to_step().0;
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
