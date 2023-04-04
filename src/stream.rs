use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{Sample, FromSample};
use std::error::Error;
use parking_lot::Mutex;
use std::sync::Arc;
use crate::AudioState;
use lazy_static::lazy_static;

lazy_static!{
    static ref AUDIO_STATE: Mutex<Option<Arc<Mutex<AudioState>>>> = Mutex::new(None);
    static ref R_AUDIO_STATE: Mutex<Option<Arc<Mutex<AudioState>>>> = Mutex::new(None);
}

pub fn prepare_cpal_stream(audio: Arc<Mutex<AudioState>>, r_audio: Arc<Mutex<AudioState>>) -> Result<cpal::Stream, Box<dyn Error>> {
    *AUDIO_STATE.lock() = Some(audio);
    *R_AUDIO_STATE.lock() = Some(r_audio);

    let host = cpal::default_host();
    let device = host.default_output_device().ok_or("No output device available")?;

    let mut supported_configs_range = device.supported_output_configs()
        .map_err(|e| format!("Error while querying configs: {}", e))?;
    let supported_config = supported_configs_range.next()
        .ok_or("No supported config found")?
        .with_max_sample_rate();

    let stream = device.build_output_stream(
        &supported_config.into(),
        move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
            write_audio(data)
        },
        move |err| {
            // react to errors here.
        },
        None // None=blocking, Some(Duration)=timeout
    ).map_err(|e| format!("Error while building output stream: {}", e))?;
    stream.play().map_err(|e| format!("Failed to play stream: {}", e))?;

    Ok(stream)
}

#[inline]
fn write_audio<T: Sample + FromSample<f32>>(data: &mut [T]) {
    for sample in data.iter_mut() {
        *sample = Sample::EQUILIBRIUM;
    }

    let state = AUDIO_STATE.lock();
    let mut audio = state.as_ref().unwrap().lock();
    if audio.playing {
        audio.add_audio(data);
    }
    std::mem::drop(audio);
    std::mem::drop(state);
    
    let state = R_AUDIO_STATE.lock();
    let mut audio = state.as_ref().unwrap().lock();
    if audio.playing {
        audio.add_audio(data);
    }
    std::mem::drop(audio);
    std::mem::drop(state);
}