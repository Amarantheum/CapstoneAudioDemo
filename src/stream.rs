use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{Sample, FromSample};
use std::error::Error;
use lazy_static::lazy_static;
use parking_lot::Mutex;

pub static mut LOC: usize = 0;
lazy_static! {
    pub static ref AUDIO: Mutex<[Vec<f32>; 2]> = Mutex::new([vec![], vec![]]);
    pub static ref P: Mutex<bool> = Mutex::new(false);
}
pub fn prepare_cpal_stream() -> Result<cpal::Stream, Box<dyn Error>> {
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
            write_silence(data)
        },
        move |err| {
            // react to errors here.
        },
        None // None=blocking, Some(Duration)=timeout
    ).map_err(|e| format!("Error while building output stream: {}", e))?;
    stream.play().map_err(|e| format!("Failed to play stream: {}", e))?;

    Ok(stream)
}

fn write_audio<T: Sample + FromSample<f32>>(data: &mut [T]) {
    let audio = AUDIO.lock();
    let mut cur = 0;
    if *P.lock() {
        for sample in data.iter_mut() {
            *sample = unsafe { audio[cur][LOC].to_sample() };
            cur = (cur + 1) % 2;
            if cur == 0 {
                unsafe { LOC += 1; }
            }
        }
    } else {
        for sample in data.iter_mut() {
            *sample = Sample::EQUILIBRIUM;
        }
    }
    
}