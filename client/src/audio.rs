//! Microphone capture + Opus encoding, and remote Opus decoding + playback.
//!
//! Audio is *always* sent to the relay server as base64 Opus packets. The
//! client never opens a peer-to-peer socket and never learns a peer's address.
//!
//! Threading model:
//! * All `cpal::Stream` objects live on one dedicated `std::thread` (streams
//!   may be `!Send`, and device enumeration must never stall the UI).
//! * The callback -> encoder path uses a bounded Tokio channel, so a slow
//!   encoder can never grow memory without limit.
//! * Opus encoding runs in a short-lived Tokio task owned by `AudioEngine`;
//!   stopping moves `abort()` + join to a cleanup thread so egui never blocks.

use base64::Engine;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use skypeline_shared::WsClientMsg;
use std::cell::RefCell;
use std::collections::VecDeque;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{mpsc as smpsc, Arc, Mutex};
use std::thread::JoinHandle;
use std::time::Duration;
use tokio::runtime::Runtime;
use tokio::sync::mpsc as tmpsc;
use tokio::sync::Notify;
use tokio::task::JoinHandle as TokioJoinHandle;

const OPUS_FRAME: usize = 960; // 20 ms at 48 kHz
const OPUS_RATE: u32 = 48_000;
const MAX_ENCODED_PACKET: usize = 1275;
const PCM_FRAME_CAP: usize = 64; // ~1.3 s of 20 ms frames
const CONTROL_CAP: usize = 8;
const CAPTURE_START_TIMEOUT: Duration = Duration::from_secs(3);
const PLAYBACK_SECONDS: usize = 2;
const CAPTURE_BUFFER_CAP: usize = OPUS_FRAME * 4;

/// Commands sent from the UI thread to the thread that owns the cpal streams.
enum AudioCommand {
    StartCapture {
        pcm_tx: tmpsc::Sender<Vec<f32>>,
        stop: Arc<AtomicBool>,
        reply: smpsc::SyncSender<Result<(), String>>,
    },
    StopCapture,
    Shutdown,
}

#[derive(Default)]
struct PlaybackState {
    /// 48 kHz mono PCM waiting to be resampled by the output callback.
    queue: VecDeque<f32>,
    /// Fractional position between `previous` and `queue.front()`.
    phase: f64,
    previous: f32,
    /// Sample rate reported by the output device (48 kHz until configured).
    out_rate: u32,
}

impl PlaybackState {
    fn push_pcm(&mut self, pcm: &[f32]) {
        let max = (OPUS_RATE as usize).saturating_mul(PLAYBACK_SECONDS);
        for &sample in pcm {
            let sample = if sample.is_finite() {
                sample.clamp(-1.0, 1.0)
            } else {
                0.0
            };
            self.queue.push_back(sample);
        }
        if self.queue.len() > max {
            let excess = self.queue.len() - max;
            self.queue.drain(..excess);
        }
    }
}

thread_local! {
    // Scratch buffers keep the audio callbacks allocation-free after warm-up.
    // Format conversion and downmixing use separate cells so the I16/U16
    // callbacks never borrow the same RefCell twice.
    static OUTPUT_SCRATCH: RefCell<Vec<f32>> = const { RefCell::new(Vec::new()) };
    static INPUT_CONVERTED: RefCell<Vec<f32>> = const { RefCell::new(Vec::new()) };
    static INPUT_MONO: RefCell<Vec<f32>> = const { RefCell::new(Vec::new()) };
    static INPUT_RESAMPLED: RefCell<Vec<f32>> = const { RefCell::new(Vec::new()) };
}

/// Owns the audio threads and the short-lived Opus encoding task.
pub struct AudioEngine {
    control_tx: smpsc::SyncSender<AudioCommand>,
    audio_thread: Option<JoinHandle<()>>,
    playback: Arc<Mutex<PlaybackState>>,
    muted: Arc<AtomicBool>,
    deafened: Arc<AtomicBool>,
    capture_stop: Arc<AtomicBool>,
    capture_notify: Option<Arc<Notify>>,
    capture_ws: Option<tmpsc::UnboundedSender<WsClientMsg>>,
    capture_rt: Option<Runtime>,
    capture_task: Option<TokioJoinHandle<()>>,
}

impl AudioEngine {
    /// Start the playback stream. Microphone capture starts only on Join Voice.
    pub fn start() -> Self {
        let (control_tx, control_rx) = smpsc::sync_channel(CONTROL_CAP);
        let playback = Arc::new(Mutex::new(PlaybackState {
            out_rate: OPUS_RATE,
            ..PlaybackState::default()
        }));

        let thread_playback = Arc::clone(&playback);
        let audio_thread = std::thread::Builder::new()
            .name("skypeline-audio".to_string())
            .spawn(move || audio_thread_main(control_rx, thread_playback))
            .map_err(|e| tracing::warn!("cannot start audio thread: {e}"))
            .ok();

        Self {
            control_tx,
            audio_thread,
            playback,
            muted: Arc::new(AtomicBool::new(false)),
            deafened: Arc::new(AtomicBool::new(false)),
            capture_stop: Arc::new(AtomicBool::new(true)),
            capture_notify: None,
            capture_ws: None,
            capture_rt: None,
            capture_task: None,
        }
    }

    /// Enqueue decoded 48 kHz mono PCM from a remote participant.
    pub fn push_pcm(&self, pcm: &[f32]) {
        let mut state = self
            .playback
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        state.push_pcm(pcm);
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
        // Never leave two input streams (or two encoders) running at once.
        self.stop_capture();

        let (pcm_tx, mut pcm_rx) = tmpsc::channel(PCM_FRAME_CAP);
        let stop = Arc::new(AtomicBool::new(false));
        let notify = Arc::new(Notify::new());
        let (reply_tx, reply_rx) = smpsc::sync_channel(1);

        send_control(
            &self.control_tx,
            AudioCommand::StartCapture {
                pcm_tx,
                stop: Arc::clone(&stop),
                reply: reply_tx,
            },
        )?;

        match reply_rx.recv_timeout(CAPTURE_START_TIMEOUT) {
            Ok(Ok(())) => {}
            Ok(Err(e)) => return Err(e),
            Err(_) => {
                stop.store(true, Ordering::SeqCst);
                let _ = send_control(&self.control_tx, AudioCommand::StopCapture);
                return Err("timed out while opening the microphone".to_string());
            }
        }

        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(|e| {
                stop.store(true, Ordering::SeqCst);
                let _ = send_control(&self.control_tx, AudioCommand::StopCapture);
                format!("cannot start audio encoding task: {e}")
            })?;

        let control_tx = self.control_tx.clone();
        let task_stop = Arc::clone(&stop);
        let task_notify = Arc::clone(&notify);
        let muted = Arc::clone(&self.muted);
        let deafened = Arc::clone(&self.deafened);
        let task_ws = ws.clone();

        let capture_task = rt.spawn(async move {
            let mut encoder = match opus1::OpusEncoder::new(
                opus1::SampleRate::Hz48000,
                opus1::Channels::Mono,
                opus1::Application::Voip,
            ) {
                Ok(encoder) => encoder,
                Err(e) => {
                    tracing::warn!("Opus encoder unavailable: {e}");
                    task_stop.store(true, Ordering::SeqCst);
                    let _ = send_control(&control_tx, AudioCommand::StopCapture);
                    task_notify.notify_one();
                    let _ = task_ws.send(WsClientMsg::SetVoiceState {
                        muted: muted.load(Ordering::Relaxed),
                        deafened: deafened.load(Ordering::Relaxed),
                        speaking: false,
                    });
                    return;
                }
            };

            let mut speaking = false;
            let mut packet = [0u8; MAX_ENCODED_PACKET];

            loop {
                let frame = tokio::select! {
                    _ = task_notify.notified() => {
                        if task_stop.load(Ordering::SeqCst) {
                            break;
                        }
                        continue;
                    }
                    incoming = pcm_rx.recv() => {
                        match incoming {
                            Some(frame) => frame,
                            None => {
                                let _ = task_ws.send(WsClientMsg::SetVoiceState {
                                    muted: muted.load(Ordering::Relaxed),
                                    deafened: deafened.load(Ordering::Relaxed),
                                    speaking: false,
                                });
                                break;
                            }
                        }
                    }
                };

                // Defensive: only ever hand a complete 20 ms frame to Opus.
                if frame.len() != OPUS_FRAME {
                    continue;
                }

                let silent = muted.load(Ordering::Relaxed) || deafened.load(Ordering::Relaxed);
                let active = !silent && crate::native::audio_peak(&frame) > 0.012;
                if active != speaking {
                    speaking = active;
                    let _ = task_ws.send(WsClientMsg::SetVoiceState {
                        muted: muted.load(Ordering::Relaxed),
                        deafened: deafened.load(Ordering::Relaxed),
                        speaking,
                    });
                }
                if silent {
                    continue;
                }

                let encoded = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    encoder.encode(&frame, OPUS_FRAME, &mut packet)
                }));
                match encoded {
                    Ok(Ok(len)) if len > 0 && len <= packet.len() => {
                        let opus_b64 =
                            base64::engine::general_purpose::STANDARD.encode(&packet[..len]);
                        if task_ws
                            .send(WsClientMsg::VoicePacket {
                                room: room.clone(),
                                opus_b64,
                            })
                            .is_err()
                        {
                            break;
                        }
                    }
                    Ok(Err(e)) => tracing::warn!("Opus encode error: {e}"),
                    Err(_) => {
                        tracing::warn!("Opus encoder panicked; stopping capture");
                        task_stop.store(true, Ordering::SeqCst);
                        let _ = send_control(&control_tx, AudioCommand::StopCapture);
                        task_notify.notify_one();
                        break;
                    }
                    _ => tracing::warn!("Opus encoder returned an invalid packet length"),
                }
            }
        });

        self.capture_stop = stop;
        self.capture_notify = Some(notify);
        self.capture_ws = Some(ws);
        self.capture_rt = Some(rt);
        self.capture_task = Some(capture_task);
        Ok(())
    }

    pub fn stop_capture(&mut self) {
        let was_active = !self.capture_stop.swap(true, Ordering::SeqCst);

        // Publish the speaking=false transition even if the task is aborted.
        if was_active {
            if let Some(ws) = &self.capture_ws {
                let _ = ws.send(WsClientMsg::SetVoiceState {
                    muted: self.muted.load(Ordering::Relaxed),
                    deafened: self.deafened.load(Ordering::Relaxed),
                    speaking: false,
                });
            }
        }
        self.capture_ws = None;
        if let Some(notify) = &self.capture_notify {
            notify.notify_one();
        }
        self.capture_notify = None;
        let _ = send_control(&self.control_tx, AudioCommand::StopCapture);

        if let Some(task) = self.capture_task.take() {
            task.abort();
            // Joining a Tokio task needs a runtime; do it off the UI thread.
            let rt = self.capture_rt.take();
            if let Some(rt) = rt {
                if std::thread::Builder::new()
                    .name("skypeline-capture-cleanup".to_string())
                    .spawn(move || {
                        let _ = rt.block_on(task);
                    })
                    .is_err()
                {
                    tracing::warn!("could not spawn capture cleanup thread");
                }
            }
        }
        // If there was no task there is nothing to keep alive.
        self.capture_rt = None;
    }
}

impl Drop for AudioEngine {
    fn drop(&mut self) {
        self.stop_capture();
        let _ = send_control(&self.control_tx, AudioCommand::Shutdown);
        if let Some(thread) = self.audio_thread.take() {
            // Join on a helper thread: a stuck device close must not block egui.
            let _ = std::thread::Builder::new()
                .name("skypeline-audio-join".to_string())
                .spawn(move || {
                    let _ = thread.join();
                });
        }
    }
}

/// Owns every `cpal::Stream`; dropping it here drops streams on this thread.
fn audio_thread_main(
    control_rx: smpsc::Receiver<AudioCommand>,
    playback: Arc<Mutex<PlaybackState>>,
) {
    let host = cpal::default_host();

    // No output device is not fatal: voice capture still works.
    if let Some((device, config)) = select_output(&host) {
        match build_output(&device, &config, &playback) {
            Ok(stream) => match stream.play() {
                Ok(()) => {}
                Err(e) => tracing::warn!("cannot start playback stream: {e}"),
            },
            Err(e) => tracing::warn!("playback disabled: {e}"),
        }
    } else {
        tracing::warn!("playback disabled: no output device available");
    }

    let mut input: Option<cpal::Stream> = None;
    while let Ok(command) = control_rx.recv() {
        match command {
            AudioCommand::StartCapture {
                pcm_tx,
                stop,
                reply,
            } => {
                input.take();
                let result = select_input(&host).and_then(|(device, config)| {
                    let buffer = Arc::new(Mutex::new(Vec::<f32>::new()));
                    build_input(&device, &config, pcm_tx, stop, buffer)
                });
                match &result {
                    Ok(stream) => match stream.play() {
                        Ok(()) => {}
                        Err(e) => {
                            let message = format!("cannot start microphone: {e}");
                            tracing::warn!("{message}");
                            let _ = reply.send(Err(message));
                            continue;
                        }
                    },
                    Err(e) => tracing::warn!("microphone unavailable: {e}"),
                }
                let ok = result
                    .map(|stream| {
                        input = Some(stream);
                    })
                    .map_err(|e| e);
                let _ = reply.send(ok);
            }
            AudioCommand::StopCapture => {
                input.take();
            }
            AudioCommand::Shutdown => break,
        }
    }
    input.take();
    // Output stream is dropped here as well, on its owning thread.
}

fn select_input(host: &cpal::Host) -> Result<(cpal::Device, cpal::SupportedStreamConfig), String> {
    if let Some(device) = host.default_input_device() {
        if let Ok(config) = device.default_input_config() {
            return Ok((device, config));
        }
        tracing::warn!("default microphone has no usable config; trying other inputs");
    }
    let devices = host
        .input_devices()
        .map_err(|e| format!("cannot enumerate microphones: {e}"))?;
    for device in devices {
        if let Ok(config) = device.default_input_config() {
            return Ok((device, config));
        }
    }
    Err("no microphone detected (no usable input device)".to_string())
}

fn select_output(host: &cpal::Host) -> Option<(cpal::Device, cpal::SupportedStreamConfig)> {
    if let Some(device) = host.default_output_device() {
        if let Ok(config) = device.default_output_config() {
            return Some((device, config));
        }
    }
    let devices = host.output_devices().ok()?;
    for device in devices {
        if let Ok(config) = device.default_output_config() {
            return Some((device, config));
        }
    }
    None
}

fn build_input(
    device: &cpal::Device,
    config: &cpal::SupportedStreamConfig,
    pcm_tx: tmpsc::Sender<Vec<f32>>,
    stop: Arc<AtomicBool>,
    buffer: Arc<Mutex<Vec<f32>>>,
) -> Result<cpal::Stream, String> {
    let channels = config.channels() as usize;
    let rate = config.sample_rate();
    let stream_config = config.config();

    let stream = match config.sample_format() {
        cpal::SampleFormat::F32 => device.build_input_stream(
            stream_config,
            move |data: &[f32], _| {
                if stop.load(Ordering::Relaxed) {
                    return;
                }
                capture_samples(data, channels, rate, &buffer, &pcm_tx);
            },
            audio_error,
            None,
        ),
        cpal::SampleFormat::I16 => device.build_input_stream(
            stream_config,
            move |data: &[i16], _| {
                if stop.load(Ordering::Relaxed) {
                    return;
                }
                INPUT_CONVERTED.with(|cell| {
                    let mut converted = cell.borrow_mut();
                    converted.resize(data.len(), 0.0);
                    for (dst, src) in converted.iter_mut().zip(data.iter()) {
                        *dst = (*src as f32) / 32_768.0;
                    }
                    capture_samples(&converted, channels, rate, &buffer, &pcm_tx);
                });
            },
            audio_error,
            None,
        ),
        cpal::SampleFormat::U16 => device.build_input_stream(
            stream_config,
            move |data: &[u16], _| {
                if stop.load(Ordering::Relaxed) {
                    return;
                }
                INPUT_CONVERTED.with(|cell| {
                    let mut converted = cell.borrow_mut();
                    converted.resize(data.len(), 0.0);
                    for (dst, src) in converted.iter_mut().zip(data.iter()) {
                        *dst = (*src as f32 - 32_768.0) / 32_768.0;
                    }
                    capture_samples(&converted, channels, rate, &buffer, &pcm_tx);
                });
            },
            audio_error,
            None,
        ),
        other => return Err(format!("unsupported microphone sample format: {other:?}")),
    };
    stream.map_err(|e| format!("cannot open microphone: {e}"))
}

fn build_output(
    device: &cpal::Device,
    config: &cpal::SupportedStreamConfig,
    playback: &Arc<Mutex<PlaybackState>>,
) -> Result<cpal::Stream, String> {
    let channels = config.channels().max(1) as usize;
    let rate = config.sample_rate().max(1);
    {
        let mut state = playback
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        state.out_rate = rate;
    }
    let stream_config = config.config();
    // The callbacks outlive this function, so they need owned Arcs.
    let stream_playback = Arc::clone(playback);

    let stream = match config.sample_format() {
        cpal::SampleFormat::F32 => device.build_output_stream(
            stream_config,
            move |data: &mut [f32], _| {
                render_output(data, channels, &stream_playback);
            },
            audio_error,
            None,
        ),
        cpal::SampleFormat::I16 => device.build_output_stream(
            stream_config,
            move |data: &mut [i16], _| {
                render_output(data, channels, &stream_playback);
            },
            audio_error,
            None,
        ),
        cpal::SampleFormat::U16 => device.build_output_stream(
            stream_config,
            move |data: &mut [u16], _| {
                render_output(data, channels, &stream_playback);
            },
            audio_error,
            None,
        ),
        other => return Err(format!("unsupported playback sample format: {other:?}")),
    };
    stream.map_err(|e| format!("cannot open playback device: {e}"))
}

/// Fill an output buffer, resampling 48 kHz mono to the device rate and
/// duplicating the mono sample into every requested channel.
fn render_output<T>(data: &mut [T], channels: usize, playback: &Arc<Mutex<PlaybackState>>)
where
    T: OutputSample,
{
    let frames = data.len() / channels.max(1);
    let gain = 1.0f32;

    OUTPUT_SCRATCH.with(|cell| {
        let mut scratch = cell.borrow_mut();
        scratch.resize(frames, 0.0);
        {
            let mut state = playback
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            let step = OPUS_RATE as f64 / state.out_rate.max(1) as f64;
            for sample in scratch.iter_mut() {
                *sample = take_mono(&mut state, step) * gain;
            }
        }
        for (frame, value) in scratch.iter().enumerate() {
            let value = value.clamp(-1.0, 1.0);
            let converted = T::from_mono(value);
            let start = frame * channels;
            for slot in data[start..].iter_mut().take(channels) {
                *slot = converted;
            }
        }
    });
}

/// Tiny local trait so the three supported cpal sample formats share the
/// resampling/interleaving code without any extra dependency.
trait OutputSample: Copy {
    fn from_mono(value: f32) -> Self;
}

impl OutputSample for f32 {
    fn from_mono(value: f32) -> Self {
        value
    }
}

impl OutputSample for i16 {
    fn from_mono(value: f32) -> Self {
        (value * 32_767.0) as i16
    }
}

impl OutputSample for u16 {
    fn from_mono(value: f32) -> Self {
        ((value + 1.0) * 32_767.5).round() as u16
    }
}

/// Streaming linear interpolation: `phase` tracks the fractional source
/// position between the previously consumed sample and the queue head.
fn take_mono(state: &mut PlaybackState, step: f64) -> f32 {
    let next = state.queue.front().copied().unwrap_or(0.0);
    let value = state.previous * (1.0 - state.phase as f32) + next * state.phase as f32;
    state.phase += step;
    while state.phase >= 1.0 {
        state.phase -= 1.0;
        state.previous = state.queue.pop_front().unwrap_or(0.0);
    }
    value
}

fn audio_error<E: std::fmt::Display>(err: E) {
    tracing::warn!("audio stream error: {err}");
}

/// Non-blocking control-channel send. If the (tiny) queue is momentarily full,
/// hand the command to a helper thread instead of stalling the UI.
fn send_control(tx: &smpsc::SyncSender<AudioCommand>, command: AudioCommand) -> Result<(), String> {
    match tx.try_send(command) {
        Ok(()) => Ok(()),
        Err(smpsc::TrySendError::Full(command)) => {
            let tx = tx.clone();
            std::thread::Builder::new()
                .name("skypeline-audio-control".to_string())
                .spawn(move || {
                    let _ = tx.send(command);
                })
                .map_err(|e| format!("cannot queue audio command: {e}"))?;
            Ok(())
        }
        Err(smpsc::TrySendError::Disconnected(_)) => Err("audio thread is unavailable".to_string()),
    }
}

/// Downmix interleaved input to mono, resample to 48 kHz, and enqueue complete
/// 20 ms frames. The channel is bounded and `try_send` never blocks cpal.
fn capture_samples(
    data: &[f32],
    channels: usize,
    device_rate: u32,
    buffer: &Arc<Mutex<Vec<f32>>>,
    tx: &tmpsc::Sender<Vec<f32>>,
) {
    if channels == 0 || device_rate == 0 {
        return;
    }
    let frames = data.len() / channels;
    if frames == 0 {
        return;
    }

    INPUT_MONO.with(|mono_cell| {
        let mut mono = mono_cell.borrow_mut();
        mono.resize(frames, 0.0);
        for (frame_index, output) in mono.iter_mut().enumerate() {
            let mut sum = 0.0f32;
            for sample in &data[frame_index * channels..(frame_index + 1) * channels] {
                sum += if sample.is_finite() { *sample } else { 0.0 };
            }
            *output = sum / channels as f32;
        }

        INPUT_RESAMPLED.with(|resampled_cell| {
            let mut resampled = resampled_cell.borrow_mut();
            resample_into(&mono, device_rate as f64, OPUS_RATE as f64, &mut resampled);

            let mut pending = buffer
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            pending.extend_from_slice(&resampled);
            if pending.len() > CAPTURE_BUFFER_CAP {
                let excess = pending.len() - CAPTURE_BUFFER_CAP;
                pending.drain(..excess);
            }
            while pending.len() >= OPUS_FRAME {
                let frame: Vec<f32> = pending.drain(..OPUS_FRAME).collect();
                // Full channel: drop this (newest) frame rather than blocking
                // the audio callback or growing memory without bound.
                if tx.try_send(frame).is_err() {
                    tracing::trace!("Opus frame queue full; dropping frame");
                }
            }
        });
    });
}

/// Linear-interpolation resampler (good enough for voice, no dependencies).
fn resample_into(input: &[f32], from: f64, to: f64, out: &mut Vec<f32>) {
    out.clear();
    if input.is_empty() || from <= 0.0 || to <= 0.0 {
        return;
    }
    if (from - to).abs() < 1.0 {
        out.extend_from_slice(input);
        return;
    }
    let ratio = to / from;
    let out_len = ((input.len() as f64) * ratio).round().max(1.0) as usize;
    let out_len = out_len.min(input.len().saturating_mul(8).saturating_add(16));
    out.resize(out_len, 0.0);
    for (index, slot) in out.iter_mut().enumerate() {
        let position = index as f64 / ratio;
        let low = (position.floor() as usize).min(input.len() - 1);
        let high = (low + 1).min(input.len() - 1);
        let frac = (position - low as f64) as f32;
        *slot = input[low] * (1.0 - frac) + input[high] * frac;
    }
}
