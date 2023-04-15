use std::path::Path;
use std::error::Error;
use cpal::{Sample, FromSample};
use gp_resonator::{resonator_array::ConjPoleResonatorArray, resonator::ConjPoleResonator};
use resonator_builder::scaled_builder::ScaledResonatorPlan;

use crate::load_audio;

pub struct AudioState {
    audio: [Vec<f32>; 2],
    pub playing: bool,
    loc: usize,

    pub filter: Option<(ConjPoleResonatorArray, ConjPoleResonatorArray)>,
    pub decay: f64,
    pub old_decay: f64,

    pub plan: Option<ScaledResonatorPlan>,
    pub transpose: f64,
    pub old_transpose: f64,

    pub sample_rate: f64,
    pub limiter_scale: f64,
    pub volume: f64,
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
                decay: 1.0,
                old_decay: 1.0,
                plan: None,
                transpose: 0.0,
                old_transpose: 0.0,
                limiter_scale: 0.0,
                volume: 0.0,
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
            if self.old_decay != self.decay {
                self.old_decay = self.decay;
                f1.set_resonator_decays(4_f64.powf(self.decay) - 1.0);
                f2.set_resonator_decays(4_f64.powf(self.decay) - 1.0);
            }
            if self.transpose != self.old_transpose {
                self.old_transpose = self.transpose;
                let trans_amt = 2_f64.powf(self.transpose);
                let plan = self.plan.as_ref().unwrap();
                let transpose_fn = |index: usize, res: &mut ConjPoleResonator| {
                    res.set_arg(plan.resonators[index].0 * trans_amt);
                };
                f1.update_resonators(transpose_fn);
                f2.update_resonators(transpose_fn);
            }
            let mut audio1 = Vec::with_capacity(buf_size);
            let mut audio2 = Vec::with_capacity(buf_size);
            let mut chan1 = vec![0.0; buf_size];
            let mut chan2 = vec![0.0; buf_size];
            for _ in 0..buf_size {
                audio1.push(self.audio[0][self.loc] as f64);
                audio2.push(self.audio[1][self.loc] as f64);
                self.loc = (self.loc + 1) % self.audio[0].len();
            }
            f1.process_buf(&audio1[..], &mut chan1[..]);
            f2.process_buf(&audio2[..], &mut chan2[..]);

            // internal limiting
            let mut max = 0.0;
            for i in 0..buf_size {
                chan1[i] *= 10_f64.powf(self.volume * 0.1);
                chan2[i] *= 10_f64.powf(self.volume * 0.1);
                if chan1[i].abs() > max {
                    max = chan1[i].abs()
                }
                if chan2[i].abs() > max {
                    max = chan2[i].abs()
                }
            }
            let prev = self.limiter_scale as f32;
            if max.log2() > self.limiter_scale {
                if max.log2() > self.limiter_scale + 2.0 {
                    self.limiter_scale = max.log2();
                } else {
                    self.limiter_scale += 0.2;
                }
                
            } else {
                self.limiter_scale -= 0.2;
            }
            self.limiter_scale = self.limiter_scale.max(-2.0);
            let new = self.limiter_scale as f32;

            for i in 0..buf_size {
                data[2 * i] += chan1[i] as f32 / 2_f32.powf((new - prev) * (i as f32 / buf_size as f32) + prev + 2.0);
                data[2 * i + 1] += chan2[i] as f32 / 2_f32.powf((new - prev) * (i as f32 / buf_size as f32) + prev + 2.0);
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