use eframe::{egui, CreationContext};
use egui::Stroke;

struct AudioPlayer {
    playing: bool,
    progress: f32,
}

impl Default for AudioPlayer {
    fn default() -> Self {
        Self {
            playing: false,
            progress: 0.0,
        }
    }
}

impl AudioPlayer {
    fn new(_cc: &CreationContext<'_>) -> Self {
        Self::default()
    }
}

impl eframe::App for AudioPlayer {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        let Self { playing, progress } = self;

        egui::CentralPanel::default().show(ctx, |ui| {
            if ui.button(if *playing { "⏸" } else { "▶" }).clicked() {
                *playing = !*playing;
            }

            let (response, painter) = ui.allocate_painter(ui.available_size_before_wrap(), egui::Sense::click());
            painter.rect(
                response.rect,
                2.0,
                ui.visuals().extreme_bg_color,
                Stroke::default(),
            );
            painter.rect(
                egui::Rect::from_min_size(
                    response.rect.min,
                    egui::vec2(response.rect.width() * *progress, response.rect.height()),
                ),
                2.0,
                ui.visuals().selection.bg_fill,
                Stroke::default(),
            );

            if response.clicked() {
                let click_pos = response.interact_pointer_pos().unwrap();
                let new_progress = (click_pos.x - response.rect.min.x) / response.rect.width();
                *progress = new_progress;
                println!("Clicked progress bar at position: {}", new_progress);
            }
        });

        if *playing {
            if *progress > 1.0 {
                *progress = 0.0;
            }
        }
    }
}

fn main() {
    let native_options = eframe::NativeOptions::default();
    eframe::run_native("AudioPlayer", native_options, Box::new(|cc| Box::new(AudioPlayer::new(cc)))).expect("failed to start");
}