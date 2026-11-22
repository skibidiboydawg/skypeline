//! Microphone capture + Opus encoding, and remote Opus decoding + playback.
//!
//! Audio is *always* sent to the relay server as base64 Opus packets. The
//! client never opens a peer-to-peer socket and never learns a peer's address.

use base64::Engine;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use skypeline_shared::WsClientMsg;
use std::collections::VecDeque;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc as tmpsc;
use tokio::task::JoinHandle;

const OPUS_FRAME: usize = 960; // 20 ms at 48 kHz
const OPUS_RATE: f32 = 48_000.0;

#[derive(Default)]
struct PlaybackState {
    queue: VecDeque<f32>,
}

/// Owns the live cpal streams; dropping it stops capture and playback.
pub struct AudioEngine {
    _input: Option<cpal::Stream>,
    _output: Option<cpal::Stream>,
    playback: Arc<Mutex<PlaybackState>>,
    muted: Arc<AtomicBool>,
    deafened: Arc<AtomicBool>,
    capture_stop: Arc<AtomicBool>,
    capture_task: Option<JoinHandle<()>>,
    out_rate: f32,
}

impl AudioEngine {
    /// Start the playback stream. Microphone capture starts only on Join Voice.
    pub fn start() -> Self {
        let host = cpal::default_host();
        let playback = Arc::new(Mutex::new(PlaybackState::default()));
        let mut out_rate = OPUS_RATE;

        let output = host.default_output_device().and_then(|device| {
            let cfg = device.default_output_config().ok()?;
            out_rate = cfg.sample_rate() as f32;
            let stream_cfg = cpal::StreamConfig {
                channels: cfg.channels(),
                sample_rate: cfg.sample_rate(),
                buffer_size: cpal::BufferSize::Default,
            };
            let state = Arc::clone(&playback);
            let stream = match cfg.sample_format() {
                cpal::SampleFormat::F32 => device.build_output_stream(
                    stream_cfg,
                    move |data: &mut [f32], _| fill_output(data, &state),
                    audio_error,
                    None,
                ),
                cpal::SampleFormat::I16 => device.build_output_stream(
                    stream_cfg,
                    move |data: &mut [i16], _| {
                        let mut tmp = vec![0.0f32; data.len()];
                        fill_output(&mut tmp, &state);
                        for (dst, src) in data.iter_mut().zip(tmp) {
                            *dst = (src.clamp(-1.0, 1.0) * 32_767.0) as i16;
                        }
                    },
                    audio_error,
                    None,
                ),
                _ => return None,
            };
            stream.ok()
        });

        if let Some(stream) = &output {
            let _ = stream.play();
        }

        Self {
            _input: None,
            _output: output,
            playback,
            muted: Arc::new(AtomicBool::new(false)),
            deafened: Arc::new(AtomicBool::new(false)),
            capture_stop: Arc::new(AtomicBool::new(true)),
            capture_task: None,
            out_rate,
        }
    }

    /// Enqueue decoded 48 kHz mono PCM from a remote participant.
    pub fn push_pcm(&self, pcm: &[f32]) {
        let resampled = resample(pcm, OPUS_RATE, self.out_rate);
        let mut state = self.playback.lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
        let max = (self.out_rate as usize) * 2;
        if state.queue.len() > max {
            state.queue.drain(..max / 2);
        }
        state.queue.extend(resampled);
    }

    pub fn set_muted(&self, value: bool) {
        self.muted.store(value, Ordering::Relaxed);
    }

    pub fn set_deafened(&self, value: bool) {
        self.deafened.store(value, Ordering::Relaxed);
    }

    pub fn start_capture(
        &mut self,
        room: String,
        ws: tmpsc::UnboundedSender<WsClientMsg>,
    ) -> Result<(), String> {
        self.stop_capture();

        let host = cpal::default_host();
        let device = host
            .default_input_device()
            .ok_or_else(|| "no microphone detected".to_string())?;
        let cfg = device
            .default_input_config()
            .map_err(|e| format!("microphone config error: {e}"))?;
        let device_rate = cfg.sample_rate();
        let stream_cfg = cpal::StreamConfig {
            channels: cfg.channels(),
            sample_rate: cfg.sample_rate(),
            buffer_size: cpal::BufferSize::Default,
        };

        let (pcm_tx, mut pcm_rx) = tmpsc::unbounded_channel::<Vec<f32>>();
        let buffer = Arc::new(Mutex::new(Vec::<f32>::new()));

        let stream = match cfg.sample_format() {
            cpal::SampleFormat::F32 => {
                let buffer = Arc::clone(&buffer);
                let tx = pcm_tx.clone();
                device.build_input_stream(
                    stream_cfg,
                    move |data: &[f32], _| capture_samples(data, device_rate, &buffer, &tx),
                    audio_error,
                    None,
                )
            }
            cpal::SampleFormat::I16 => {
                let buffer = Arc::clone(&buffer);
                let tx = pcm_tx.clone();
                device.build_input_stream(
                    stream_cfg,
                    move |data: &[i16], _| {
                        let samples: Vec<f32> = data
                            .iter()
                            .map(|s| (*s as f32) / 32_768.0)
                            .collect();
                        capture_samples(&samples, device_rate, &buffer, &tx);
                    },
                    audio_error,
                    None,
                )
            }
            _ => return Err("unsupported microphone sample format".to_string()),
        }
        .map_err(|e| format!("cannot open microphone: {e}"))?;

        stream
            .play()
            .map_err(|e| format!("cannot start microphone: {e}"))?;
        self._input = Some(stream);

        let stop = Arc::clone(&self.capture_stop);
        stop.store(false, Ordering::Relaxed);
        let muted = Arc::clone(&self.muted);
        let deafened = Arc::clone(&self.deafened);
        let ws_for_task = ws.clone();

        self.capture_task = Some(tokio::spawn(async move {
            let mut encoder = match opus1::OpusEncoder::new(
                opus1::SampleRate::Hz48000,
                opus1::Channels::Mono,
                opus1::Application::Voip,
            ) {
                Ok(encoder) => encoder,
                Err(e) => {
                    let _ = ws_for_task.send(WsClientMsg::SetVoiceState {
                        muted: false,
                        deafened: false,
                        speaking: false,
                    });
                    tracing::warn!("Opus encoder unavailable: {e}");
                    return;
                }
            };
            let mut speaking = false;
            let mut packet = [0u8; 1275];

            while !stop.load(Ordering::Relaxed) {
                let Some(frame) = pcm_rx.recv().await else { break };
                let silent = muted.load(Ordering::Relaxed) || deafened.load(Ordering::Relaxed);
                let active = !silent && crate::native::audio_peak(&frame) > 0.012;
                if active != speaking {
                    speaking = active;
                    let _ = ws_for_task.send(WsClientMsg::SetVoiceState {
                        muted: muted.load(Ordering::Relaxed),
                        deafened: deafened.load(Ordering::Relaxed),
                        speaking,
                    });
                }
                if silent {
                    continue;
                }
                if let Ok(len) = encoder.encode(&frame, OPUS_FRAME, &mut packet) {
                    let opus_b64 = base64::engine::general_purpose::STANDARD
                        .encode(&packet[..len]);
                    let _ = ws_for_task.send(WsClientMsg::VoicePacket {
                        room: room.clone(),
                        opus_b64,
                    });
                }
            }

            let _ = ws_for_task.send(WsClientMsg::SetVoiceState {
                muted: muted.load(Ordering::Relaxed),
                deafened: deafened.load(Ordering::Relaxed),
                speaking: false,
            });
        }));

        Ok(())
    }

    pub fn stop_capture(&mut self) {
        self.capture_stop.store(true, Ordering::Relaxed);
        if let Some(task) = self.capture_task.take() {
            task.abort();
        }
        self._input = None;
    }
}

impl Drop for AudioEngine {
    fn drop(&mut self) {
        self.stop_capture();
    }
}

fn audio_error<E: std::fmt::Display>(err: E) {
    tracing::warn!("audio stream error: {err}");
}

fn fill_output(data: &mut [f32], state: &Arc<Mutex<PlaybackState>>) {
    let mut state = state.lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    for sample in data.iter_mut() {
        *sample = state.queue.pop_front().unwrap_or(0.0);
    }
}

fn capture_samples(
    data: &[f32],
    device_rate: u32,
    buffer: &Arc<Mutex<Vec<f32>>>,
    tx: &tmpsc::UnboundedSender<Vec<f32>>,
) {
    let resampled = resample(data, device_rate as f32, OPUS_RATE);
    let mut buffer = buffer.lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    buffer.extend_from_slice(&resampled);
    while buffer.len() >= OPUS_FRAME {
        let frame: Vec<f32> = buffer.drain(..OPUS_FRAME).collect();
        let _ = tx.send(frame);
    }
}

/// Linear interpolation resampler (good enough for voice, no dependencies).
fn resample(input: &[f32], from: f32, to: f32) -> Vec<f32> {
    if input.is_empty() {
        return Vec::new();
    }
    if (from - to).abs() < 1.0 {
        return input.to_vec();
    }
    let ratio = to / from;
    let out_len = ((input.len() as f32) * ratio).round().max(1.0) as usize;
    let mut out = Vec::with_capacity(out_len);
    for i in 0..out_len {
        let pos = i as f32 / ratio;
        let lo = (pos.floor() as usize).min(input.len() - 1);
        let hi = (lo + 1).min(input.len() - 1);
        let frac = pos - lo as f32;
        out.push(input[lo] * (1.0 - frac) + input[hi] * frac);
    }
    out
}
