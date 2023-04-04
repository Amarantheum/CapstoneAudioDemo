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
    #[data(ignore)]
    progress: ProgressBar,
    #[data(ignore)]
    r_progress: ProgressBar,
    playing: bool,
    r_playing: bool,
    #[data(ignore)]
    audio_state: Arc<Mutex<AudioState>>,
    #[data(ignore)]
    r_audio_state: Arc<Mutex<AudioState>>,
    line_graph: GraphData,
}

#[derive(Clone)]
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
    
    let stream = prepare_cpal_stream(Arc::clone(&audio), Arc::clone(&r_audio))?;
    let state = AppState {
        progress: ProgressBar::init(Arc::clone(&audio)),
        r_progress: ProgressBar::init(Arc::clone(&r_audio)),
        playing: false,
        r_playing: false,
        audio_state: audio,
        r_audio_state: r_audio,
        line_graph: GraphData::new("./audio/Datmosphere.wav")?,
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

    let build_button = Label::new("BUILD RESONATOR")
    .with_text_size(24.0)
    .padding(10.0)
    .background(Painter::new(|ctx, _data: &AppState, _env| {
        let bounds = ctx.size().to_rect();
        ctx.fill(bounds, &Color::rgb8(0x7B, 0x61, 0x9E));
    }))
    .on_click(|_ctx, data: &mut AppState, _env| {
        let plan = data.line_graph.plan.lock();
        println!("{:?}", plan);
        let mut audio_state = data.audio_state.lock();
        let array = match plan.build_resonator_array(audio_state.sample_rate) {
            Ok(v) => {
                let v1 = v.clone();
                let v2 = v;
                Some((v1, v2))
            },
            Err(e) => {
                println!("Error occurred while building resonator array: {:?}", e);
                None
            }
        };
        audio_state.filter = array;
    });

    let progress_bar = SizedBox::new(CustomProgressBar.lens(AppState::progress)).height(24.0);
    let label = Label::new(|_data: &AppState, _env: &_| {
        "./audio/static_memories.wav"
    });

    let max_peaks_label = Label::new("max peaks");
    let max_peaks_lens = AppState::line_graph.then(GraphData::max_peaks);
    let max_peaks_slider = Slider::new()
        .with_range(0.0, 1.0)
        .with_step(0.0001)
        .track_color(druid::KeyOrValue::Concrete(Color::rgb8(0x7B, 0x61, 0x9E)))
        .knob_style(druid::widget::KnobStyle::Circle)
        .axis(Axis::Vertical)
        .lens(max_peaks_lens);

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
                        .with_child(max_peaks_label)
                        .with_child(max_peaks_slider) 
                )
                .with_spacer(8.0)
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
                .with_spacer(8.0)
                .with_child(build_button)
            
        )

}

#[inline]
fn load_audio<P: AsRef<Path>>(path: P) -> Result<([Vec<f32>; 2], f64), Box<dyn Error>> {
    let file = File::open(path)?;
    let source = Decoder::new(BufReader::new(file))?;
    let sample_rate = source.sample_rate() as f64;
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
    Ok((samples, sample_rate))
}