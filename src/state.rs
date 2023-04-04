use std::path::Path;
use std::error::Error;
use cpal::{Sample, FromSample};
use gp_resonator::resonator_array::ConjPoleResonatorArray;

use crate::load_audio;

pub struct AudioState {
    audio: [Vec<f32>; 2],
    pub playing: bool,
    loc: usize,

    pub filter: Option<(ConjPoleResonatorArray, ConjPoleResonatorArray)>,
    pub sample_rate: f64,
}

impl AudioState {
    #[inline]
    pub fn init_audio_state<P: AsRef<Path>>(path: P) -> Result<AudioState, Box<dyn Error>> {
        let (audio, sample_rate) = load_audio(path)?;
        Ok(
            AudioState {
                audio,
                playing: false,
                loc: 0,
                filter: None,
                sample_rate,
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
    pub fn add_audio(&mut self, data: &mut [f32]) {
        let buf_size = data.len() / 2;
        if let Some((f1, f2)) = self.filter.as_mut() {
            let mut audio1 = Vec::with_capacity(buf_size);
            let mut audio2 = Vec::with_capacity(buf_size);
            let mut chan1 = vec![0.0; buf_size];
            let mut chan2 = vec![0.0; buf_size];
            for i in 0..buf_size {
                audio1.push(self.audio[0][self.loc] as f64);
                audio2.push(self.audio[1][self.loc] as f64);
                self.loc = (self.loc + 1) % self.audio[0].len();
            }
            f1.process_buf(&audio1[..], &mut chan1[..]);
            f2.process_buf(&audio2[..], &mut chan2[..]);

            for i in 0..buf_size {
                data[2 * i] += chan1[i] as f32 / 100.0;
                data[2 * i + 1] += chan2[i] as f32 / 100.0;
            }
        } else {
            for i in 0..buf_size {
                data[2 * i] += self.audio[0][self.loc];
                data[2 * i + 1] += self.audio[1][self.loc];
                self.loc = (self.loc + 1) % self.audio[0].len();
            }
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