mod command;
mod response;

use crate::response::Response;
use command::Command;
use eframe::NativeOptions;
use egui::Slider;
use egui_extras::Size;
use egui_extras::StripBuilder;
use egui_extras::TableBuilder;
use futures::FutureExt;
use itertools::Itertools;
use smol::io::AsyncBufReadExt;
use smol::io::AsyncWriteExt;
use smol::io::BufReader;
use smol::net::TcpStream;
use std::sync::Arc;
use tracing::{error, info};

#[derive(Clone, Debug)]
enum Message {
    Connected,
    Disconnected,
    Command(Command),
    ReceivedParameters(Vec<(Arc<String>, f64)>),
}

async fn setup_socket(ip: &str, mut channels: Channels) {
    info!("Setting up tcp socket...");
    let mut socket = TcpStream::connect(ip).await.unwrap();
    channels
        .message_tx
        .broadcast(Message::Connected)
        .await
        .unwrap();
    let mut reader = BufReader::new(socket.clone());
    info!("TCP r/w set up. Listening...");
    loop {
        let mut buf = String::new();
        futures::select! {
            read = reader.read_line(&mut buf).fuse() => {
                match read {
                    Ok(n) => {
                        match serde_json::from_str::<Response>(&buf) {
                            Ok(response) => match response {
                                Response::Parameters(parameters) => {
                                    let parameters = parameters.into_iter().map(|(k, v)| (Arc::new(k), v)).sorted_by_key(|(k, _)| k.clone()).collect::<Vec<(Arc<String>, f64)>>();
                                    channels.message_tx.broadcast(Message::ReceivedParameters(parameters)).await.unwrap();
                                }
                                Response::Done => {}
                            }
                            Err(err) => {
                                error!("{}", err);
                                channels.message_tx.broadcast(Message::Disconnected).await.unwrap();
                                break;
                            }
                        }
                        info!("{} bytes read", n);
                    },
                    Err(err) => {
                        error!("{}", err);
                    },
                }
            }
            send = channels.message_rx.recv().fuse() => {
                if let Ok(Message::Command(command)) = send {
                    let msg = serde_json::to_string(&command).unwrap();
                    socket.write_all(msg.as_bytes()).await.unwrap();
                }
            }
            _ = channels.shutdown_rx.recv().fuse() => {
                info!("Shutting down socket thread.");
                channels.message_tx.broadcast(Message::Disconnected).await.unwrap();
                break;
            }
        }
    }
    info!("Exiting the socket loop...");
}

struct Channels {
    message_tx: async_broadcast::Sender<Message>,
    message_rx: async_broadcast::Receiver<Message>,
    shutdown_rx: async_broadcast::Receiver<()>,
    shutdown_tx: async_broadcast::Sender<()>,
}

impl Clone for Channels {
    fn clone(&self) -> Self {
        Self {
            message_tx: self.message_tx.clone(),
            message_rx: self.message_rx.new_receiver(),
            shutdown_rx: self.shutdown_rx.new_receiver(),
            shutdown_tx: self.shutdown_tx.clone(),
        }
    }
}

impl Channels {
    pub fn new() -> Channels {
        let (mut message_tx, mut message_rx) = async_broadcast::broadcast(100);
        let (mut shutdown_tx, mut shutdown_rx) = async_broadcast::broadcast(100);

        message_tx.set_overflow(true);
        message_rx.set_overflow(true);
        shutdown_rx.set_overflow(true);
        shutdown_tx.set_overflow(true);

        Channels {
            message_tx,
            message_rx,
            shutdown_tx,
            shutdown_rx,
        }
    }
}

struct App {
    channels: Channels,
    connected: bool,
    parameters: Vec<(Arc<String>, f64)>,
}

impl App {
    pub fn new() -> App {
        App {
            channels: Channels::new(),
            parameters: Vec::new(),
            connected: false,
        }
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        while let Ok(message) = self.channels.message_rx.try_recv() {
            match message {
                Message::Connected => self.connected = true,
                Message::Disconnected => self.connected = false,
                Message::ReceivedParameters(parameters) => self.parameters = parameters,
                _ => {}
            }
        }
        egui::CentralPanel::default().show(ctx, |ui| {
            if self.connected {
                ui.horizontal(|ui| {
                    if ui.button("Disconnect").clicked() {
                        smol::block_on(self.channels.shutdown_tx.broadcast(())).unwrap();
                    }
                    if ui.button("Refresh parameters").clicked() {
                        smol::block_on(
                            self.channels
                                .message_tx
                                .broadcast(Message::Command(Command::ListParameters)),
                        )
                        .unwrap();
                    }
                });
                StripBuilder::new(ui)
                    .size(Size::remainder().at_least(100.0))
                    .vertical(|mut strip| {
                        strip.cell(|ui| {
                            egui::ScrollArea::horizontal().show(ui, |ui| {
                                TableBuilder::new(ui)
                                    .striped(true)
                                    .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
                                    .column(Size::remainder().at_least(100.0))
                                    .column(Size::remainder().at_least(100.0))
                                    .column(Size::remainder().at_least(100.0))
                                    .resizable(true)
                                    .header(20.0, |mut header| {
                                        header.col(|ui| {
                                            ui.heading("Parameter Name");
                                        });
                                        header.col(|ui| {
                                            ui.heading("Parameter Value");
                                        });
                                        header.col(|ui| {
                                            ui.heading("Update Value");
                                        });
                                    })
                                    .body(|body| {
                                        body.rows(
                                            30.0,
                                            self.parameters.len(),
                                            |row_index, mut row| {
                                                row.col(|ui| {
                                                    ui.label(self.parameters[row_index].0.as_str());
                                                });
                                                row.col(|ui| {
                                                    ui.add(Slider::new(
                                                        &mut self.parameters[row_index].1,
                                                        0.0..=100.0,
                                                    ));
                                                });
                                                row.col(|ui| {
                                                    if ui.button("Update").clicked() {
                                                        smol::block_on(
                                                            self.channels.message_tx.broadcast(
                                                                Message::Command(
                                                                    Command::SetParameterValue {
                                                                        name: self.parameters
                                                                            [row_index]
                                                                            .0
                                                                            .to_string(),
                                                                        value: self.parameters
                                                                            [row_index]
                                                                            .1,
                                                                    },
                                                                ),
                                                            ),
                                                        )
                                                        .unwrap();
                                                    }
                                                });
                                            },
                                        );
                                    });
                            });
                        })
                    });
            } else if ui.button("Connect").clicked() {
                smol::spawn(setup_socket("127.0.0.1:5000", self.channels.clone())).detach();
            }
        });
    }
}

fn main() {
    tracing_subscriber::fmt::init();
    info!("Starting up...");

    let icon = include_bytes!("../resources/icon.png");

    let options = NativeOptions {
        icon_data: Some(eframe::IconData {
            rgba: icon.to_vec(),
            width: 32,
            height: 32,
        }),
        ..Default::default()
    };

    eframe::run_native(
        "Test Async App",
        options,
        Box::new(|_cc| Box::new(App::new())),
    );
}
