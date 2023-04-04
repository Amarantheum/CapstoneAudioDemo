use cpal::Sample;
use druid::widget::prelude::*;
use druid::{Data, Lens, Color, Rect};
use druid::kurbo::{BezPath, Point, Circle};
use resonator_builder::scaled_builder::*;
use parking_lot::Mutex;
use std::sync::Arc;
use resonator_builder::fft::{FftCalculator, window::WindowFunction};
use std::path::Path;
use crate::load_audio;
use std::error::Error;

const MAX_PEAKS: usize = 2000;

#[derive(Clone, Data, Lens)]
pub struct GraphData {
    // values between 0.0 and 1.0
    #[data(ignore)]
    pub spec: Vec<f64>,

    #[data(ignore)]
    pub audio: Vec<f64>,
    pub sample_rate: f64,

    // value between 0.0 and 1.0
    pub min_line: f64,

    #[data(ignore)]
    pub plan: Arc<Mutex<ScaledResonatorPlan>>,

    pub min_range: f64,
    pub max_range: f64,

    pub min_prominence: f64,
    pub max_peaks: f64,

    // the difference between the highest and lowest value in the spectrum
    pub spectrum_scale: f64,
    // the lowest value displayed in the spectrum
    pub spectrum_base: f64,
}

impl GraphData {
    #[inline]
    pub fn value_to_pixel(height: f64, value: f64) -> f64 {
        height - value * height
    }

    pub fn new<P: AsRef<Path>>(path: P) -> Result<Self, Box<dyn Error>> {
        let (audio, spec, spectrum_scale, spectrum_base, sample_rate) = load_resonant_audio(path, 1000)?;
        Ok(
            Self {
                spec,
                audio,
                sample_rate,

                min_line: 0.0,
                plan: Arc::new(Mutex::new(ScaledResonatorPlan::empty())),

                min_range: 0.0,
                max_range: 0.5,
                min_prominence: 0.3,
                max_peaks: 0.1,

                spectrum_base,
                spectrum_scale,
            }
        )
    }
}

#[inline]
fn load_resonant_audio<P: AsRef<Path>>(path: P, resolution: usize) -> Result<(Vec<f64>, Vec<f64>, f64, f64, f64), Box<dyn Error>> {
    let ([chan1, chan2], sample_rate) = load_audio(path)?;
    let audio = chan1.into_iter().zip(chan2.into_iter()).map(|v| ((v.0 + v.1) / 2.0) as f64).collect::<Vec<f64>>();
    let near_pow_2 = ((audio.len() - 1).ilog2() + 1) as usize;
    let fft_size = 2_usize.pow(near_pow_2 as u32);
    let mut fft = FftCalculator::new(audio.len(), fft_size - audio.len())?;
    let comp_freqs = fft.real_fft(&audio[..], resonator_builder::fft::window::Rectangular::real_window);
    
    let freqs = comp_freqs[0..fft_size / 2]
        .into_iter()
        .map(|v| v.norm().log10())
        .collect::<Vec<f64>>();
    
    let mut global_max = f64::MIN;
    for v in &freqs {
        if *v > global_max {
            global_max = *v;
        }
    }
    let min_value = -3.0;
    let scale = global_max - min_value;
    let mut cur_bin = 0;
    let mut out = Vec::with_capacity(resolution);
    for i in 0..resolution {
        let mut max = f64::MIN;
        while (cur_bin as f64 / freqs.len() as f64) < ((i + 1) as f64 / resolution as f64) {
            if max < freqs[cur_bin] {
                max = freqs[cur_bin];
            }
            cur_bin += 1;
        }
        out.push((max.max(min_value) - min_value) / scale)
    }
    Ok((audio, out, scale, min_value, sample_rate))
}

// a custom widget that draws a line graph
pub struct LineGraph;

impl LineGraph {
    // create a new instance of the widget
    fn new() -> Self {
        Self
    }
}

impl Widget<GraphData> for LineGraph {
    fn event(&mut self, _ctx: &mut EventCtx, event: &Event, _data: &mut GraphData, _env: &Env) {
    }

    fn lifecycle(&mut self, _ctx: &mut LifeCycleCtx, _event: &LifeCycle, _data: &GraphData, _env: &Env) {}

    fn update(&mut self, ctx: &mut UpdateCtx, old_data: &GraphData, data: &GraphData, _env: &Env) {
        // request a repaint when the data changes
        if !old_data.same(data) {
            ctx.request_paint();
        }
        
    }

    fn layout(&mut self, _ctx: &mut LayoutCtx, bc: &BoxConstraints, _data: &GraphData, _env: &Env) -> Size {
        // use the maximum size available
        bc.max()
    }

    fn paint(&mut self, ctx: &mut PaintCtx, data: &GraphData, env: &Env) {
        println!("repaint");
        // get the size of the widget
        let size = ctx.size();

        let bg = Rect::new(0.0, 0.0, size.width, size.height);
        ctx.fill(bg, &Color::rgb8(0, 0, 0));

        // create a color for the line graph
        let color = Color::rgb8(0x1e, 0xcb, 0xe1);

        let grey = Color::grey(0.8);
        // create a path for the line graph
        let mut path = BezPath::new();

        // move to the first point of the line graph
        if let Some(first) = data.spec.first() {
            path.move_to(Point::new(0.0, GraphData::value_to_pixel(size.height, *first)));
        }

        // add lines to the rest of the points of the line graph
        for (i, value) in data.spec.iter().enumerate().skip(1) {
            path.line_to(Point::new(i as f64 * size.width / (data.spec.len() - 1) as f64, GraphData::value_to_pixel(size.height, *value)));
        }

        // stroke the path with some thickness
        ctx.stroke(path, &color, 2.0);

        let mut path = BezPath::new();
        path.move_to(Point::new(0.0, GraphData::value_to_pixel(size.height, data.min_line)));
        path.line_to(Point::new(size.width, GraphData::value_to_pixel(size.height, data.min_line)));
        ctx.stroke(path, &grey, 2.0);

        let plan;
        if data.min_range >= data.max_range {
            plan = ScaledResonatorPlan::empty();
        } else {
            plan = ScaledResonatorPlanner::new()
                .with_min_prominence(data.min_prominence * data.spectrum_scale)
                .with_max_num_peaks((data.max_peaks * MAX_PEAKS as f64) as usize)
                .with_min_freq(data.min_range)
                .with_max_freq(data.max_range)
                .with_min_threshold(data.min_line * data.spectrum_scale + data.spectrum_base)
                .plan(&data.audio[..], data.sample_rate);
        }

        for peak in &plan.resonators {
            let x = peak.0 / std::f64::consts::PI;
            let y = (x * data.spec.len() as f64) as usize;
            let circle = Circle::new(Point::new(x * size.width, GraphData::value_to_pixel(size.height, data.spec[y])), 5.0);
            ctx.fill(circle, &Color::rgba8(255, 255, 255, 64))
        }

        let mut new_plan = data.plan.lock();
        *new_plan = plan;
        std::mem::drop(new_plan);

        let mut path = BezPath::new();
        path.move_to(Point::new(data.min_range * size.width, 0.0));
        path.line_to(Point::new(data.min_range * size.width, size.height));
        ctx.stroke(path, &grey, 2.0);

        let mut path = BezPath::new();
        path.move_to(Point::new(data.max_range * size.width, 0.0));
        path.line_to(Point::new(data.max_range * size.width, size.height));
        ctx.stroke(path, &grey, 2.0);

        let selected = Rect::new( data.min_range * size.width, 0.0, data.max_range * size.width, GraphData::value_to_pixel(size.height, data.min_line));
        ctx.fill(selected, &Color::rgba8(255, 255, 255, 20));
    }
}