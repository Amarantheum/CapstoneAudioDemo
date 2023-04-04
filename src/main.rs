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
use rodio::{Decoder, source::Source};
use state::AudioState;
use lazy_static::lazy_static;
use crate::stream::prepare_cpal_stream;
use resonator_builder::fft::FftCalculator;

mod stream;
mod state;
mod graph;

lazy_static!{
}

#[derive(Clone, Data, Lens)]
struct AppState {
    progress: ProgressBar,
    r_progress: ProgressBar,
    playing: bool,
    r_playing: bool,
    audio_state: Arc<Mutex<AudioState>>,
    r_audio_state: Arc<Mutex<AudioState>>,
    line_graph: GraphData,
}

#[derive(Clone, Lens)]
struct ProgressBar {
    audio: Arc<Mutex<AudioState>>,
}

impl Data for ProgressBar {
    fn same(&self, _other: &Self) -> bool {
        // stub fow now
        false
    }
}

impl ProgressBar {
    pub fn init(audio: Arc<Mutex<AudioState>>) -> Self {
        Self {
            audio,
        }
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
                    let progress = (x / width).max(0.0).min(1.0);
                    audio.set_loc(progress);
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
        let audio = data.audio.lock();
        let progress = audio.get_progress();
        let filled_rect = Rect::from_origin_size((0.0, 0.0), (size.width * progress, size.height));
        std::mem::drop(audio);
        ctx.fill(rect, &Color::grey(1.0));
        ctx.fill(filled_rect, &Color::rgb8(0x7B, 0x61, 0x9E));
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<String> = env::args().collect();
    println!("{:?}", args);
    let window = WindowDesc::new(build_ui());
    let audio = Arc::new(Mutex::new(AudioState::init_audio_state("./audio/static_memories.wav")?));
    let r_audio = Arc::new(Mutex::new(AudioState::init_audio_state("./audio/Datmosphere.wav")?));
    let (r_combined_audio, spec, scale, min_value) = load_resonant_audio("./audio/Datmosphere.wav", 1000)?;
    
    let stream = prepare_cpal_stream(Arc::clone(&audio), Arc::clone(&r_audio))?;
    let state = AppState {
        progress: ProgressBar::init(Arc::clone(&audio)),
        r_progress: ProgressBar::init(Arc::clone(&r_audio)),
        playing: false,
        r_playing: false,
        audio_state: audio,
        r_audio_state: r_audio,
        line_graph: GraphData { 
            spec: spec, 
            min_line: 0.1, 
            max_range: 0.9, 
            min_range: 0.3, 
            min_prominence: 0.5, 
            audio: r_combined_audio, 
            sample_rate: 48_000_f64, 
            max_peaks: 100, 
            spectrum_scale: scale, 
            spectrum_base: min_value
        },
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
    .on_click(|_ctx, data: &mut AppState, _env| {
        data.playing = !data.playing;
        data.audio_state.lock().playing = data.playing;
    });

    let progress_bar = SizedBox::new(CustomProgressBar.lens(AppState::progress)).height(24.0);
    let label = Label::new(|_data: &AppState, _env: &_| {
        "./audio/static_memories.wav"
    });

    let r_play_pause_button = Label::new(|data: &AppState, _env: &_| {
        if data.r_playing {
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
    .on_click(|_ctx, data: &mut AppState, _env| {
        data.r_playing = !data.r_playing;
        data.r_audio_state.lock().playing = data.r_playing;
    });

    let r_progress_bar = SizedBox::new(CustomProgressBar.lens(AppState::r_progress)).height(24.0);
    let r_label = Label::new(|_data: &AppState, _env: &_| {
        "./audio/Datmosphere.wav"
    });

    let graph = SizedBox::new(LineGraph.lens(AppState::line_graph)).height(400.0);

    let min_prom_label = Label::new("min prominence");
    let min_prom_lens = AppState::line_graph.then(GraphData::min_prominence);
    let min_prom_slider = Slider::new()
        .with_range(0.0, 1.0)
        .with_step(0.0001)
        .track_color(druid::KeyOrValue::Concrete(Color::rgb8(0x7B, 0x61, 0x9E)))
        .knob_style(druid::widget::KnobStyle::Circle)
        .axis(Axis::Vertical)
        .lens(min_prom_lens);

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
        .with_child(
            Flex::row()
                .with_child(r_play_pause_button)
                .with_spacer(8.0)
                .with_child(r_label),
        )
        .with_child(r_progress_bar)
        .with_spacer(8.0)
        .with_child(graph)
        .with_child(
            Flex::row()
                .with_child(
                    Flex::column()
                        .with_child(min_prom_label)
                        .with_child(min_prom_slider) 
                )
                .with_spacer(8.0)
                .with_child(
                    Flex::column()
                        .with_child(min_thresh_label)
                        .with_child(thresh_slider) 
                )
                .with_spacer(8.0)
                .with_child(
                    Flex::column()
                        .with_child(min_freq_label)
                        .with_child(min_freq_slider) 
                )
                .with_spacer(8.0)
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
fn load_resonant_audio<P: AsRef<Path>>(path: P, resolution: usize) -> Result<(Vec<f64>, Vec<f64>, f64, f64), Box<dyn Error>> {
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
    Ok((audio, out, scale, min_value))
}