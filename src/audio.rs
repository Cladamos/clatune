use cpal::{
    Device, DeviceId,
    traits::{DeviceTrait, HostTrait, StreamTrait},
};
use pitch_calc::{Hz, Letter, letter_octave_from_hz, step_from_letter_octave};
use pitch_detection::detector::PitchDetector;
use pitch_detection::detector::mcleod::McLeodDetector;
use std::sync::mpsc::Sender;

use crate::app::{AppNote, ClatuneDevice, TunerData};

#[cfg(target_os = "linux")]
fn get_host() -> cpal::Host {
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

#[cfg(not(target_os = "linux"))]
fn get_host() -> cpal::Host {
    cpal::default_host()
}

pub fn get_devices() -> Result<(Vec<ClatuneDevice>, ClatuneDevice), String> {
    fn get_channels(d: Device) -> String {
        let mut channels: String = String::new();
        if let Ok(config) = d.default_input_config() {
            channels = format!(" ({}ch)", config.channels());
        }
        channels
    }
    // TODO: Hide monitor devices from list
    // I didn't get can I do it on cpal without typing linux specific ALSA code
    // Right now I just added channels so user can see it is mono or stereo
    let host = get_host();

    let input_devices = host
        .input_devices()
        .map_err(|e| format!("Failed to get input devices: {}", e))?;

    let mut devices = Vec::new();

    for device in input_devices {
        if let Ok(desc) = device.description() {
            let name = desc.name();
            if name != "unknown" {
                devices.push(ClatuneDevice {
                    id: device.id().map_err(|e| e.to_string())?.to_string(),
                    name: format!("{}{}", name, get_channels(device)),
                });
            }
        }
    }

    if devices.is_empty() {
        return Err("No input devices found".to_string());
    }

    let default_device = match host.default_input_device() {
        Some(device) => {
            let desc = device.description().map_err(|e| e.to_string())?;
            ClatuneDevice {
                id: device.id().map_err(|e| e.to_string())?.to_string(),
                name: format!("{}{}", desc.name(), get_channels(device)),
            }
        }

        None => devices[0].clone(),
    };

    Ok((devices, default_device))
}

pub fn start_stream(
    tx: Sender<TunerData>,
    err_tx: Sender<String>,
    device_id: DeviceId,
    reference_pitch: u16,
) -> Result<cpal::Stream, String> {
    let host = get_host();
    let input_devices = host
        .input_devices()
        .map_err(|e| format!("Failed to get input devices: {}", e))?;

    let device = input_devices
        .filter_map(|d| d.id().ok().map(|id| (id, d)))
        .find(|(id, _)| *id == device_id)
        .ok_or_else(|| "Selected input device was not found".to_string())?
        .1;

    let mut supported_configs_range = device
        .supported_input_configs()
        .map_err(|e| format!("Error querying configs: {}", e))?;

    let supported_config = supported_configs_range
        .next()
        .ok_or_else(|| "No supported audio configurations found for this device".to_string())?
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
                        let lpf_alpha = 0.02;
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

                                let _ = tx.send(get_tuner_data(smoothed_freq, reference_pitch));
                            }
                        }
                    } else {
                        smoothed_freq = 0.0;
                    }
                    input_buffer.drain(0..2048);
                }
            },
            move |err| {
                let _ = err_tx.send(format!("Stream error: {}", err));
            },
            None,
        )
        .map_err(|e| format!("Failed to build input stream: {}", e))?;

    stream
        .play()
        .map_err(|e| format!("Failed to start audio playback: {}", e))?;

    Ok(stream)
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

fn get_tuner_data(frequency: f32, reference_pitch: u16) -> TunerData {
    const NOTES: [&str; 12] = [
        "C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B",
    ];

    let hz_diff = f32::from(reference_pitch) / 440.0;
    let changed_frequency = frequency * hz_diff;
    let (note, octave) = letter_octave_from_hz(changed_frequency);

    let input_step = Hz(changed_frequency).to_step().0;
    let nearest_step = step_from_letter_octave(note, octave);

    // I guess in this crate stesp are semitones
    // When I am trying calculate with steps it showed 2x more than it should
    // In normal 1 step = 2 semitones = 200 cents
    let cent_diff = ((input_step - nearest_step) * 100.0).round() as i32;

    let index = NOTES
        .iter()
        .position(|&r| r == letter_to_string(note))
        .expect("Every pitch letter should exist in NOTES array");
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
