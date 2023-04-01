use app_state_derived_lenses::audio_state;
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

#[derive(Clone, Data, Lens)]
struct AppState {
    progress: f64,
    audio_state: Arc<Mutex<AudioState>>
}

struct AudioState {
    audio: [Vec<f32>; 2],
    playing: bool,
    loc: usize,
}

struct CustomProgressBar;

impl Widget<f64> for CustomProgressBar {
    fn event(&mut self, ctx: &mut druid::EventCtx, event: &druid::Event, data: &mut f64, _env: &druid::Env) {
        
        match event {
            druid::Event::MouseDown(mouse_event) => {
                if mouse_event.button == MouseButton::Left {
                    let x = mouse_event.pos.x;
                    let width = ctx.size().width;
                    *data = (x / width).max(0.0).min(1.0);
                    ctx.set_handled();
                }
            }
            _ => {}
        }
    }

    fn lifecycle(&mut self, _ctx: &mut druid::LifeCycleCtx, _event: &druid::LifeCycle, _data: &f64, _env: &druid::Env) {}

    fn update(&mut self, ctx: &mut druid::UpdateCtx, old_data: &f64, data: &f64, _env: &druid::Env) {
        if old_data != data {
            ctx.request_paint();
        }
    }

    fn layout(&mut self, _ctx: &mut druid::LayoutCtx, bc: &druid::BoxConstraints, _data: &f64, _env: &druid::Env) -> druid::Size {
        bc.max()
    }

    fn paint(&mut self, ctx: &mut druid::PaintCtx, data: &f64, _env: &druid::Env) {
        let size = ctx.size();
        let rect = Rect::from_origin_size((0.0, 0.0), size);
        let filled_rect = Rect::from_origin_size((0.0, 0.0), (size.width * *data, size.height));
        ctx.fill(rect, &Color::grey(1.0));
        ctx.fill(filled_rect, &Color::rgb8(0x00, 0x80, 0x00));
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<String> = env::args().collect();
    println!("{:?}", args);
    let window = WindowDesc::new(build_ui());
    let audio = Arc::new(Mutex::new(init_audio_state("./audio/static_memories.wav")?));
    let state = AppState {
        progress: 0.0,
        audio_state: audio,
    };
    AppLauncher::with_window(window)
        .log_to_console()
        .launch(state)?;
    Ok(())
}

fn build_ui() -> impl druid::Widget<AppState> {
    let play_pause_button = Label::new(|data: &AppState, _env: &_| {
        if data.audio_state.lock().playing {
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
        let mut audio = data.audio_state.lock();
        audio.playing = !audio.playing;
        println!("{}", audio.playing);
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
fn init_audio_state<P: AsRef<Path>>(path: P) -> Result<AudioState, Box<dyn Error>> {
    Ok(
        AudioState {
            audio: load_audio(path)?,
            playing: false,
            loc: 0,
        }
    )
}

#[inline]
fn load_audio<P: AsRef<Path>>(path: P) -> Result<[Vec<f32>; 2], Box<dyn Error>> {
    let file = File::open(path)?;
    let source = Decoder::new(BufReader::new(file))?;
    let channels = source.channels();
    if channels != 2 {
        return Err("This app only supporst audio files with 2 channels :C".into());
    }
    let mut samples = [Vec::new(), Vec::new()];
    let mut cur = 0;
    for v in source.convert_samples::<f32>() {
        samples[cur].push(v);
        cur = (cur + 1) % 2;
    }
    Ok(samples)
}