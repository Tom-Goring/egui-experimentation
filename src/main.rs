#![windows_subsystem = "windows"]

use anyhow::Context;
use command::Command;
use eframe::NativeOptions;
use egui::{Color32, RichText, Slider};
use egui_extras::{Size, TableBuilder};

use futures::AsyncWriteExt;
use itertools::Itertools;
use poll_promise::Promise;

use response::Response;
use smol::io::AsyncReadExt;
use smol::{future, net::TcpStream, LocalExecutor};
use std::ops::Deref;
use std::{cell::RefCell, collections::HashMap, rc::Rc};
use tracing::{error, info};

mod command;
mod response;

#[derive(Default)]
struct State {
    parameters: HashMap<Rc<String>, f64>,
    socket: Option<TcpStream>,
    ip: String,
    error: Option<String>,
}

struct App {
    futures: RefCell<Vec<Promise<anyhow::Result<()>>>>,
    state: Rc<RefCell<State>>,
}

impl App {
    pub fn new() -> App {
        App {
            futures: RefCell::new(Vec::new()),
            state: Rc::new(RefCell::new(Default::default())),
        }
    }
}

impl App {
    pub fn connect(&self) {
        let state = self.state.clone();
        self.futures
            .borrow_mut()
            .push(Promise::spawn_local(async move {
                if let Ok(mut state) = state.try_borrow_mut() {
                    match TcpStream::connect(state.ip.deref()).await {
                        Ok(new_socket) => {
                            state.socket = Some(new_socket);
                        }
                        Err(err) => {
                            state.error = Some(err.to_string());
                        }
                    };
                }
                Ok(())
            }));
    }

    pub fn send(&self, command: Command) {
        let state = self.state.clone();
        self.futures
            .borrow_mut()
            .push(Promise::spawn_local(async move {
                if let Ok(mut state) = state.try_borrow_mut() {
                    if let Some(socket) = &mut state.socket {
                        let msg = serde_json::to_string(&command)?;
                        socket.write_all(msg.as_bytes()).await?;
                    }
                }

                let mut buf = [0; 2048];
                let n = state.borrow_mut().socket.as_mut().context("Somehow lost the socket connection")?.read(&mut buf).await?;
                let response: Response = serde_json::from_slice(&buf[0..n])?;
                match response {
                    Response::Parameters(parameters) => {
                        state.borrow_mut().parameters = parameters
                            .into_iter()
                            .map(|(k, v)| (Rc::new(k), v))
                            .collect();
                    }
                    Response::Done => {},
                }
                Ok(())
            }));
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        for future in self.futures.borrow().iter() {
            if let Some(Err(err)) = future.ready() {
                error!("{}", err);
            }
        }

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.label(
                RichText::new("DEBUG STB2 UI - NOT REPRESENTATIVE OF FINAL UI")
                    .color(Color32::GREEN),
            );

            if let Ok(mut state) = self.state.try_borrow_mut() {
                if let Some(error) = &state.error {
                    ui.label(RichText::new(error).color(Color32::RED));
                }
                if state.socket.is_some() {
                    ui.label("Connected!");
                    ui.horizontal(|ui| {
                        if ui.button("Refresh parameters").clicked() {
                            self.send(Command::ListParameters);
                        }
                    });
                    TableBuilder::new(ui)
                        .striped(true)
                        .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
                        .column(Size::remainder().at_least(100.0))
                        .column(Size::remainder().at_least(100.0))
                        .column(Size::remainder().at_least(100.0))
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
                        .body(|mut body| {
                            for (param_name, param_value) in state.parameters.iter_mut().sorted_by_key(|(k, _)| k.clone()) {
                                body.row(30.0, |mut row| {
                                    row.col(|ui| {
                                        ui.label(param_name.as_str());
                                    });
                                    row.col(|ui| {
                                        ui.add(Slider::new(param_value, 0.0..=100.0));
                                    });
                                    row.col(|ui| {
                                        if ui.button("Update").clicked() {
                                            self.send(Command::SetParameterValue {
                                                name: param_name.to_string(),
                                                value: *param_value,
                                            });
                                        }
                                    });
                                });
                            }
                        });
                } else {
                    ui.text_edit_singleline(&mut state.ip);
                    if ui.button("Connect").clicked() {
                        self.connect();
                    }
                }
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

    let local = LocalExecutor::new();

    future::block_on(local.run(async {
        eframe::run_native(
            "Test Async App",
            options,
            Box::new(|_cc| Box::new(App::new())),
        );
    }));
}
