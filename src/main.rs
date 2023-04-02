use druid::widget::{Flex, Label, Painter, SizedBox, Slider, Axis};
use druid::{AppLauncher, Color, Data, Lens, RenderContext, WidgetExt, WindowDesc, Widget, MouseButton, LensExt};
use druid::kurbo::Rect;
use graph::{LineGraph, GraphData};
use resonator_builder::fft::window::WindowFunction;
use std::error::Error;
use std::path::Path;
use std::sync::Arc;
use parking_lot::Mutex;
use std::env;
use std::fs::File;
use std::io::BufReader;
use rodio::{Decoder, OutputStream, source::Source};
use state::AudioState;
use lazy_static::lazy_static;
use crate::stream::prepare_cpal_stream;
use resonator_builder::fft::FftCalculator;

mod stream;
mod state;
mod graph;

lazy_static!{
    static ref PROGRESS: Arc<Mutex<f64>> = Arc::new(Mutex::new(0.0));
}

#[derive(Clone, Data, Lens)]
struct AppState {
    progress: ProgressBar,
    playing: bool,
    audio_state: Arc<Mutex<AudioState>>,
    line_graph: GraphData,
}

#[derive(Clone, Lens)]
struct ProgressBar {
    progress: Arc<Mutex<f64>>,
    audio: Arc<Mutex<AudioState>>,
}

impl Data for ProgressBar {
    fn same(&self, other: &Self) -> bool {
        let v1 = *self.progress.lock();
        let v2 = *other.progress.lock();
        v1 == v2
    }
}

impl ProgressBar {
    pub fn init(audio: Arc<Mutex<AudioState>>) -> Self {
        Self {
            progress: Arc::clone(&*PROGRESS),
            audio,
        }
    }

    fn set_progress(&mut self, v: f64) {
        *self.progress.lock() = v;
    }

    fn get_progress(&self) -> f64 {
        *self.progress.lock()
    }
}

struct CustomProgressBar;

impl Widget<ProgressBar> for CustomProgressBar {
    fn event(&mut self, ctx: &mut druid::EventCtx, event: &druid::Event, data: &mut ProgressBar, _env: &druid::Env) {
        
        match event {
            druid::Event::MouseDown(mouse_event) => {
                if mouse_event.button == MouseButton::Left {
                    let x = mouse_event.pos.x;
                    let width = ctx.size().width;
                    let mut audio = data.audio.lock();
                    let mut progress = data.progress.lock();
                    *progress = (x / width).max(0.0).min(1.0);
                    audio.set_loc(*progress);
                    std::mem::drop(progress);
                    std::mem::drop(audio);
                    ctx.set_handled();
                    ctx.request_paint();
                }
            },
            druid::Event::WindowConnected => {
                ctx.request_timer(std::time::Duration::from_secs_f64(1.0 / 60.0));
            }
            druid::Event::Timer(_) => {
                ctx.request_paint();
                ctx.request_timer(std::time::Duration::from_secs_f64(1.0 / 60.0));
            }
            _ => {}
        }
    }

    fn lifecycle(&mut self, _ctx: &mut druid::LifeCycleCtx, _event: &druid::LifeCycle, _data: &ProgressBar, _env: &druid::Env) {}

    fn update(&mut self, ctx: &mut druid::UpdateCtx, old_data: &ProgressBar, data: &ProgressBar, _env: &druid::Env) {
        if !old_data.same(data) {
            ctx.request_paint();
        }
    }

    fn layout(&mut self, _ctx: &mut druid::LayoutCtx, bc: &druid::BoxConstraints, _data: &ProgressBar, _env: &druid::Env) -> druid::Size {
        bc.max()
    }

    fn paint(&mut self, ctx: &mut druid::PaintCtx, data: &ProgressBar, _env: &druid::Env) {
        let size = ctx.size();
        let rect = Rect::from_origin_size((0.0, 0.0), size);
        let filled_rect = Rect::from_origin_size((0.0, 0.0), (size.width * data.get_progress(), size.height));
        ctx.fill(rect, &Color::grey(1.0));
        ctx.fill(filled_rect, &Color::rgb8(0x7B, 0x61, 0x9E));
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<String> = env::args().collect();
    println!("{:?}", args);
    let window = WindowDesc::new(build_ui());
    let audio = Arc::new(Mutex::new(AudioState::init_audio_state("./audio/static_memories.wav")?));
    
    let resonant_freqs = load_resonant_audio("./audio/Datmosphere.wav", 1000)?;
    
    let stream = prepare_cpal_stream(Arc::clone(&audio))?;
    let state = AppState {
        progress: ProgressBar::init(Arc::clone(&audio)),
        playing: false,
        audio_state: audio,
        line_graph: GraphData { data: resonant_freqs, min_line: 0.1, peaks: vec![], max_range: 0.9, min_range: 0.3 },
    };
    AppLauncher::with_window(window)
        .log_to_console()
        .launch(state)?;
    Ok(())
}

fn build_ui() -> impl druid::Widget<AppState> {
    let play_pause_button = Label::new(|data: &AppState, _env: &_| {
        if data.playing {
            "Pause".to_string()
        } else {
            "Play".to_string()
        }
    })
    .with_text_size(24.0)
    .padding(10.0)
    .background(Painter::new(|ctx, _data: &AppState, _env| {
        let bounds = ctx.size().to_rect();
        ctx.fill(bounds, &Color::rgb8(0x7B, 0x61, 0x9E));
    }))
    .on_click(|ctx, data: &mut AppState, _env| {
        data.playing = !data.playing;
        data.audio_state.lock().playing = data.playing;
    });

    let progress_bar = SizedBox::new(CustomProgressBar.lens(AppState::progress)).height(24.0);
    let graph = SizedBox::new(LineGraph.lens(AppState::line_graph)).height(400.0);

    let label = Label::new(|_data: &AppState, _env: &_| {
        "./audio/static_memories.wav"
    });

    let min_thresh_label = Label::new("min threshold");
    let min_line_lens = AppState::line_graph.then(GraphData::min_line);
    let thresh_slider = Slider::new()
        .with_range(0.0, 1.0)
        .with_step(0.0001)
        .track_color(druid::KeyOrValue::Concrete(Color::rgb8(0x7B, 0x61, 0x9E)))
        .knob_style(druid::widget::KnobStyle::Circle)
        .axis(Axis::Vertical)
        .lens(min_line_lens);

    let min_freq_label = Label::new("min freq");
    let min_freq_lens = AppState::line_graph.then(GraphData::min_range);
    let min_freq_slider = Slider::new()
        .with_range(0.0, 1.0)
        .with_step(0.0001)
        .track_color(druid::KeyOrValue::Concrete(Color::rgb8(0x7B, 0x61, 0x9E)))
        .knob_style(druid::widget::KnobStyle::Circle)
        .axis(Axis::Vertical)
        .lens(min_freq_lens);

    let max_freq_label = Label::new("max freq");
    let max_freq_lens = AppState::line_graph.then(GraphData::max_range);
    let max_freq_slider = Slider::new()
        .with_range(0.0, 1.0)
        .with_step(0.0001)
        .track_color(druid::KeyOrValue::Concrete(Color::rgb8(0x7B, 0x61, 0x9E)))
        .knob_style(druid::widget::KnobStyle::Circle)
        .axis(Axis::Vertical)
        .lens(max_freq_lens);

    Flex::column()
        .with_child(
            Flex::row()
                .with_child(play_pause_button)
                .with_spacer(8.0)
                .with_child(label),
        )
        .with_child(progress_bar)
        .with_spacer(8.0)
        .with_child(graph)
        .with_child(
            Flex::row()
                .with_child(
                    Flex::column()
                        .with_child(min_thresh_label)
                        .with_child(thresh_slider) 
                )
                .with_child(
                    Flex::column()
                        .with_child(min_freq_label)
                        .with_child(min_freq_slider) 
                )
                .with_child(
                    Flex::column()
                        .with_child(max_freq_label)
                        .with_child(max_freq_slider) 
                )
            
        )

}

#[inline]
fn load_audio<P: AsRef<Path>>(path: P) -> Result<[Vec<f32>; 2], Box<dyn Error>> {
    let file = File::open(path)?;
    let source = Decoder::new(BufReader::new(file))?;
    let channels = source.channels();
    if channels != 2 {
        return Err("This app only supports audio files with 2 channels :C".into());
    }
    let mut samples = [Vec::new(), Vec::new()];
    let mut cur = 0;
    for v in source.convert_samples::<f32>() {
        samples[cur].push(v);
        cur = (cur + 1) % 2;
    }
    Ok(samples)
}

#[inline]
fn load_resonant_audio<P: AsRef<Path>>(path: P, resolution: usize) -> Result<Vec<f64>, Box<dyn Error>> {
    let [chan1, chan2] = load_audio(path)?;
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
        out.push((max.max(-3.0) + 3.0) / (global_max + 3.0))
    }
    Ok(out)
}