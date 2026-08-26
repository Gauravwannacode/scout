//! Plays the alarm tone, so the audio path can be verified without waiting
//! for 06:30.
//!
//! `cargo run --bin ring`
//!
//! This exercises the same `AudioState` the scheduler uses. If this is silent,
//! the alarm will be silent too — and that is the failure worth catching on a
//! developer machine rather than in a bedroom.

use app_lib::audio::AudioState;

fn main() {
    let state = AudioState::default();

    println!("Ringing for 5 seconds — you should hear alternating beeps.");
    state.start();
    assert!(state.is_ringing(), "the alarm reported itself as not ringing");

    std::thread::sleep(std::time::Duration::from_secs(5));

    state.stop();
    // The ringing thread clears the flag as it unwinds; give it a moment.
    std::thread::sleep(std::time::Duration::from_millis(300));
    println!(
        "Stopped. still ringing: {} (expected false)",
        state.is_ringing()
    );

    println!("\nNow the session-complete chime:");
    app_lib::audio::play_chime();
    std::thread::sleep(std::time::Duration::from_secs(1));
    println!("Done.");
}
