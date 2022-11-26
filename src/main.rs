use chrono::{DateTime, Utc, Duration};
use poll_promise::Promise;
use tokio::runtime::Runtime;

async fn test() -> String {
    tokio::time::sleep(std::time::Duration::from_secs(5)).await;
    String::from("Hello")
}

struct App {
    promise: Option<Promise<String>>,
    result: Option<String>,
    time: Option<DateTime<Utc>>,
    elapsed: Option<Duration>
}

impl App {
    pub fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        Self {
            promise: None,
            result: None,
            time: None,
            elapsed: None
        }
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if let Some(start) = self.time {
            self.elapsed = Some(Utc::now() - start);
            ctx.request_repaint();
        }
        if let Some(promise) = &self.promise {
            if let Some(result) = promise.ready() {
                self.result = Some(result.to_string());
                self.time = None;
            }
        }
        egui::CentralPanel::default().show(ctx, |ui| {
            if ui.button("Click me").clicked() {
                self.time = Some(Utc::now());
                self.promise = Some(poll_promise::Promise::spawn_async(test()));
            }
            if let Some(result) = &self.result {
                ui.label(result);
            }
            if let Some(elapsed) = &self.elapsed {
                ui.label(elapsed.to_string());
            }
        });
    }
}

fn main() {
    let rt = Runtime::new().unwrap();
    rt.block_on(async {
        eframe::run_native("My app", eframe::NativeOptions::default(), Box::new(|cc| Box::new(App::new(cc))));
    });
}
