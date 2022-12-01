

use eframe::NativeOptions;
use futures::TryFutureExt;
use poll_promise::Promise;
use tokio::net::TcpStream;
use tracing::info;

mod response;

struct App {
    promise: Option<Promise<anyhow::Result<TcpStream>>>,
    socket: Option<TcpStream>
}

impl App {
    pub fn new() -> App {
        App {
            promise: None,
            socket: None
        }
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if let Some(promise) = &self.promise {
            if promise.ready().is_some() {
                if let Ok(Ok(socket)) = self.promise.take().unwrap().try_take() {
                    self.socket = Some(socket);
                }
            }
        }

        egui::CentralPanel::default().show(ctx, |ui| {
            if ui.button("Connect").clicked() {
                self.promise = Some(Promise::spawn_async(TcpStream::connect("127.0.0.1:5000").map_err(anyhow::Error::from)));
            }
        });
    }
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();
    info!("Starting up...");

    eframe::run_native("App", NativeOptions::default(), Box::new(|_cc| Box::new(App::new())))
}
