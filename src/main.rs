use druid::widget::{Flex, Label, Painter, SizedBox};
use druid::{AppLauncher, Color, Data, Lens, RenderContext, WidgetExt, WindowDesc, Widget, MouseButton};
use druid::kurbo::Rect;
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

mod stream;
mod state;

lazy_static!{
    static ref PROGRESS: Arc<Mutex<f64>> = Arc::new(Mutex::new(0.0));
}

#[derive(Clone, Data, Lens)]
struct AppState {
    progress: ProgressBar,
    playing: bool,
    audio_state: Arc<Mutex<AudioState>>,
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
    let stream = prepare_cpal_stream(Arc::clone(&audio))?;
    let state = AppState {
        progress: ProgressBar::init(Arc::clone(&audio)),
        playing: false,
        audio_state: audio,
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

    let label = Label::new(|_data: &AppState, _env: &_| {
        "Audio"
    });

    Flex::column()
        .with_child(
            Flex::row()
                .with_child(play_pause_button)
                .with_spacer(8.0)
                .with_child(label),
        )
        .with_child(progress_bar)
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