use std::path::Path;
use std::error::Error;
use cpal::{Sample, FromSample};

use crate::load_audio;

pub struct AudioState {
    audio: [Vec<f32>; 2],
    pub playing: bool,
    loc: usize,
}

impl AudioState {
    #[inline]
    pub fn init_audio_state<P: AsRef<Path>>(path: P) -> Result<AudioState, Box<dyn Error>> {
        Ok(
            AudioState {
                audio: load_audio(path)?,
                playing: false,
                loc: 0,
            }
        )
    }

    #[inline]
    pub fn write_audio<T: Sample + FromSample<f32>>(&mut self, data: &mut [T]) {
        for i in 0..data.len() / 2 {
            data[2 * i] = self.audio[0][self.loc].to_sample();
            data[2 * i + 1] = self.audio[1][self.loc].to_sample();
            self.loc = (self.loc + 1) % self.audio[0].len();
        }
    }

    #[inline]
    pub fn add_audio<T: Sample + FromSample<f32>>(&mut self, data: &mut [T]) {
        for i in 0..data.len() / 2 {
            data[2 * i] = data[2 * i].add_amp(self.audio[0][self.loc].to_sample::<T>().to_signed_sample());
            data[2 * i + 1] = data[2 * i + 1].add_amp(self.audio[1][self.loc].to_sample::<T>().to_signed_sample());
            self.loc = (self.loc + 1) % self.audio[0].len();
        }
    }

    #[inline]
    pub fn set_loc(&mut self, v: f64) {
        debug_assert!(v >= 0.0 && v <= 1.0);

        let length = self.audio[0].len();
        self.loc = (length as f64 * v) as usize;
    }

    #[inline]
    pub fn get_progress(&self) -> f64 {
        self.loc as f64 / self.audio[0].len() as f64
    }
}