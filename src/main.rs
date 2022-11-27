use std::{time::Duration, sync::Arc};

use eframe::NativeOptions;
use egui::{RichText, Color32};
use tokio::{net::TcpStream, io::AsyncWriteExt, sync::Mutex};
use tracing::{info, error, trace};
use crossbeam_channel::{unbounded, Sender, Receiver};

enum Event {
    SocketSetupDone(std::io::Result<TcpStream>),
    SendAttempted(std::io::Result<()>),
    TcpHeartbeat(std::io::Result<()>)
}

struct App {
    tx: Sender<Event>,
    rx: Receiver<Event>,

    heartbeat_tx: Sender<()>,
    heartbeat_rx: Receiver<()>,

    input: String,
    socket: Option<Arc<Mutex<TcpStream>>>,
    error: Option<String>,
}

impl App {
    pub fn new() -> App {
        let (tx, rx) = unbounded();
        let (hbtx, hbrx) = unbounded();
        App {
            tx,
            rx,
            heartbeat_tx: hbtx,
            heartbeat_rx: hbrx,
            input: String::new(),
            socket: None,
            error: None
        }
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if let Ok(event) = self.rx.try_recv() {
            self.error = None;
            match event {
                Event::SocketSetupDone(result) => match result {
                    Ok(socket) =>  {
                        self.input = String::new();
                        self.socket = Some(Arc::new(Mutex::new(socket)));
                        setup_tcp_health_checker(self.socket.as_ref().unwrap().clone(), self.tx.clone(), self.heartbeat_rx.clone(), ctx.clone());
                    },
                    Err(err) => self.error = Some(err.to_string())
                }
                Event::SendAttempted(result) => match result {
                    Ok(_) => info!("Message sent!"),
                    Err(_) => error!("Message failed to send!"),
                }
                Event::TcpHeartbeat(result) => match result {
                    Ok(_) => info!("Heartbeat all good!"),
                    Err(_) => {
                        error!("Heartbeat failed!");
                        self.heartbeat_tx.send(()).unwrap();
                        self.socket = None;
                    }
                }
            }
        }

        egui::CentralPanel::default().show(ctx, |ui| {
            if let Some(error) = &self.error {
                ui.label(RichText::new(error).color(Color32::RED));
            }

            ui.text_edit_singleline(&mut self.input);
            if self.socket.is_none() {
                if ui.button("Connect").clicked() {
                    setup_socket(self.input.clone(), self.tx.clone(), ctx.clone());
                }
            }
            else {
                ui.label("Connected!");
                if ui.button("Send").clicked() {
                    send_on_socket(self.input.clone(), self.socket.as_ref().unwrap().clone(), self.tx.clone(), ctx.clone());
                }
            }
        });
    }
}

fn setup_socket(ip: String, tx: Sender<Event>, ctx: egui::Context) {
    tokio::spawn(async move {
        let result = TcpStream::connect(ip).await;
        let _ = tx.send(Event::SocketSetupDone(result));
        ctx.request_repaint();
    });
}

fn send_on_socket(msg: String, socket: Arc<Mutex<TcpStream>>, tx: Sender<Event>, ctx: egui::Context) {
    tokio::spawn(async move {
        let mut guard = socket.lock().await;
        let result = guard.write_all(msg.as_bytes()).await;
        let _ = tx.send(Event::SendAttempted(result));
        ctx.request_repaint();
    });
}

fn setup_tcp_health_checker(socket: Arc<Mutex<TcpStream>>, tx: Sender<Event>, rx: Receiver<()>, ctx: egui::Context) {
    tokio::spawn(async move {
        info!("Setting up heartbeat task...");
        loop {
            {
                if rx.try_recv().is_ok() {
                    error!("Received shutdown signal on heartbeat task, shutting it down...");
                    break;
                }
                trace!("Checking tcp health...");
                let mut guard = socket.lock().await;
                trace!("Got socket lock");
                trace!("Sending heartbeat request...");
                let result = guard.write_all(b"heartbeat").await;
                trace!("Reporting status back to UI...");
                let _ = tx.send(Event::TcpHeartbeat(result));
            }
            tokio::time::sleep(Duration::from_secs(1)).await;
            ctx.request_repaint();
        }
    });
    info!("Heartbeat task running.");
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();
    info!("Starting up...");

    eframe::run_native("App", NativeOptions::default(), Box::new(|_cc| Box::new(App::new())))
}
