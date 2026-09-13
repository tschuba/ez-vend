use leptos::set_timeout;
use std::time::Duration;
use wasm_bindgen::JsValue;
use wasm_bindgen_futures::{spawn_local, JsFuture};
use web_sys::{AudioContext, OscillatorType};

pub fn play_error_sound() {
    let ctx = match AudioContext::new() {
        Ok(ctx) => ctx,
        Err(err) => {
            leptos::logging::warn!("Failed to create audio context: {:?}", err);
            return;
        }
    };

    let resume_promise = match ctx.resume() {
        Ok(p) => p,
        Err(err) => {
            leptos::logging::warn!("Failed to resume audio context: {:?}", err);
            return;
        }
    };

    spawn_local(async move {
        if JsFuture::from(resume_promise).await.is_err() {
            return;
        }
        if let Err(err) = schedule_audio(ctx) {
            leptos::logging::warn!("Failed to schedule audio: {:?}", err);
        }
    });
}

fn schedule_audio(context: AudioContext) -> Result<(), JsValue> {
    let oscillator = context.create_oscillator()?;
    let gain = context.create_gain()?;

    let oscillator_for_cleanup = oscillator.clone();
    let gain_for_cleanup = gain.clone();

    oscillator.set_type(OscillatorType::Triangle);
    oscillator.frequency().set_value(392.0);

    let now = context.current_time();
    gain.gain().set_value_at_time(0.0001, now)?;
    gain.gain().linear_ramp_to_value_at_time(0.16, now + 0.01)?;
    gain.gain()
        .exponential_ramp_to_value_at_time(0.0001, now + 0.16)?;

    oscillator.connect_with_audio_node(&gain)?;
    gain.connect_with_audio_node(&context.destination())?;

    oscillator.start()?;
    oscillator.stop_with_when(now + 0.16)?;

    set_timeout(
        move || {
            let _ = oscillator_for_cleanup.disconnect();
            let _ = gain_for_cleanup.disconnect();
        },
        Duration::from_millis(250),
    );

    Ok(())
}
