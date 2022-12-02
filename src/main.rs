use command::Command;
use eframe::NativeOptions;
use futures::FutureExt;
use futures::AsyncReadExt;
use smol::io::AsyncWriteExt;
use smol::net::TcpStream;
use tracing::{info, error};

mod command;

#[derive(Clone, Debug)]
enum Message {
    Connected,
    Disconnected,
    Command(Command),
}

async fn setup_socket(ip: &str, mut channels: Channels) {
    info!("Setting up tcp socket...");
    let mut socket = TcpStream::connect(ip).await.unwrap();
    channels.message_tx.broadcast(Message::Connected).await.unwrap();
    let mut buf = [0; 8192];
    loop {
        futures::select! {
            read_result = socket.read(&mut buf).fuse() => {
                match read_result.map(|n| String::from_utf8(buf[0..n].to_vec())) {
                    Ok(Ok(msg)) => {
                        info!("{}", msg);
                    },
                    Ok(Err(err)) => {
                        error!("{}", err);
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
        Self { message_tx: self.message_tx.clone(), message_rx: self.message_rx.new_receiver(), shutdown_rx: self.shutdown_rx.new_receiver(), shutdown_tx: self.shutdown_tx.clone() }
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
    connected: bool
}



impl App {
    pub fn new() -> App {
        App {
            channels: Channels::new(),
            connected: false,
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
        while let Ok(message) = self.channels.message_rx.try_recv() {
            match message {
                Message::Connected => self.connected = true,
                Message::Disconnected => self.connected = false,
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
                        smol::block_on(self.channels.message_tx.broadcast(Message::Command(Command::ListParameters))).unwrap();
                    }
                });

            }
            else if ui.button("Connect").clicked() {
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

    let local = LocalExecutor::new();

    future::block_on(local.run(async {
        eframe::run_native(
            "Test Async App",
            options,
            Box::new(|_cc| Box::new(App::new())),
        );
    }));
}
