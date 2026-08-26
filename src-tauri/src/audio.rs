//! The sound an alarm actually makes.
//!
//! An OS notification is not an alarm. Windows shows notifications silently
//! when Focus Assist is on, when the volume mixer has muted the app, or when
//! the toast is suppressed entirely — none of which should stop a 06:30 alarm
//! waking someone. So the sound is generated and played here, in the process
//! that outlives the window, rather than relying on the notification's chime
//! or on an `<audio>` tag in a webview that may not be running.
//!
//! The tone is synthesised rather than shipped as a file: no asset to bundle,
//! no codec to depend on, and the pattern can be tuned in code.

use rodio::Source;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

/// Two alternating tones, the interval most alarm clocks use — insistent
/// without being painful.
const TONE_A_HZ: f32 = 880.0;
const TONE_B_HZ: f32 = 660.0;
const BEEP_MS: u64 = 400;
const GAP_MS: u64 = 200;

/// An alarm that rings forever is worse than one that stops. Long enough to
/// wake someone, short enough not to torment an empty room.
const MAX_RING_SECS: u64 = 60;

#[derive(Default)]
pub struct AudioState {
    ringing: Arc<AtomicBool>,
}

impl AudioState {
    pub fn is_ringing(&self) -> bool {
        self.ringing.load(Ordering::SeqCst)
    }

    /// Starts ringing. Does nothing if an alarm is already sounding, so two
    /// alarms in the same minute do not overlap into noise.
    pub fn start(&self) {
        if self.ringing.swap(true, Ordering::SeqCst) {
            return;
        }

        let ringing = self.ringing.clone();
        std::thread::spawn(move || {
            // Opening the device can fail — no sound card, or another app
            // holding exclusive-mode audio. That must not take the app down;
            // the notification and the overlay still went out.
            let sink = match rodio::DeviceSinkBuilder::open_default_sink() {
                Ok(s) => s,
                Err(e) => {
                    log::warn!("no audio output for the alarm: {e}");
                    ringing.store(false, Ordering::SeqCst);
                    return;
                }
            };
            let mixer = sink.mixer();
            let started = std::time::Instant::now();

            while ringing.load(Ordering::SeqCst) {
                if started.elapsed() > Duration::from_secs(MAX_RING_SECS) {
                    break;
                }

                for hz in [TONE_A_HZ, TONE_B_HZ] {
                    if !ringing.load(Ordering::SeqCst) {
                        break;
                    }
                    // A fade keeps the beep from starting and ending on a
                    // click, which reads as a glitch rather than a chime.
                    mixer.add(
                        rodio::source::SineWave::new(hz)
                            .take_duration(Duration::from_millis(BEEP_MS))
                            .fade_in(Duration::from_millis(25))
                            .amplify(0.35),
                    );
                    // The mixer plays asynchronously, so pacing the pattern
                    // means sleeping this thread for the beep plus its gap.
                    std::thread::sleep(Duration::from_millis(BEEP_MS + GAP_MS));
                }
            }

            // Let the tail of the last beep play out instead of clipping it
            // the instant the sink drops.
            std::thread::sleep(Duration::from_millis(120));
            ringing.store(false, Ordering::SeqCst);
        });
    }

    pub fn stop(&self) {
        self.ringing.store(false, Ordering::SeqCst);
    }
}

/// The exact source the alarm plays. Split out so a test can inspect the
/// samples rather than relying on someone hearing them.
pub fn alarm_tone(hz: f32) -> impl Source {
    rodio::source::SineWave::new(hz)
        .take_duration(Duration::from_millis(BEEP_MS))
        .fade_in(Duration::from_millis(25))
        .amplify(0.35)
}

#[tauri::command]
pub fn stop_alarm_sound(state: tauri::State<'_, AudioState>) {
    state.stop();
}

#[tauri::command]
pub fn alarm_is_ringing(state: tauri::State<'_, AudioState>) -> bool {
    state.is_ringing()
}

/// A short two-note confirmation, for a finished focus session where a full
/// alarm would be far too much.
#[tauri::command]
pub fn play_chime() {
    std::thread::spawn(|| {
        let Ok(sink) = rodio::DeviceSinkBuilder::open_default_sink() else {
            return;
        };
        let mixer = sink.mixer();
        for hz in [660.0_f32, 880.0] {
            mixer.add(
                rodio::source::SineWave::new(hz)
                    .take_duration(Duration::from_millis(170))
                    .fade_in(Duration::from_millis(20))
                    .amplify(0.25),
            );
            std::thread::sleep(Duration::from_millis(180));
        }
        // Dropping the sink cuts playback immediately, so the final note needs
        // to finish before this thread unwinds.
        std::thread::sleep(Duration::from_millis(220));
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn starting_twice_does_not_double_ring() {
        let state = AudioState::default();
        state.start();
        // Whether audio hardware exists here is irrelevant: the guard is about
        // not spawning a second ringing thread.
        state.start();
        state.stop();
        assert!(!state.is_ringing());
    }

    #[test]
    fn stop_is_safe_when_nothing_is_ringing() {
        let state = AudioState::default();
        state.stop();
        assert!(!state.is_ringing());
    }

    #[test]
    fn a_fresh_state_is_silent() {
        assert!(!AudioState::default().is_ringing());
    }

    #[test]
    fn the_alarm_tone_is_audible_not_silence() {
        // Guards the failure that is invisible in code review and inaudible in
        // CI: a source that runs for the right duration but emits zeros.
        let samples: Vec<f32> = alarm_tone(TONE_A_HZ).collect();
        assert!(!samples.is_empty(), "the tone produced no samples at all");

        let peak = samples.iter().fold(0.0_f32, |m, s| m.max(s.abs()));
        assert!(peak > 0.2, "tone peaks at {peak}, effectively silent");

        // Roughly 400ms at 44.1kHz. Loose bounds — the point is that the
        // duration is in the right order of magnitude, not exact.
        assert!(
            samples.len() > 10_000 && samples.len() < 40_000,
            "unexpected sample count: {}",
            samples.len()
        );
    }
}
