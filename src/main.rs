use eframe::egui;
use egui::plot::{Line, Plot, PlotPoints};

use core::time;
use std::{thread, sync::{Mutex, Arc}};

struct Inner {
    amplitude: f64,
}

impl Inner {
    pub fn new(amplitude: f64) -> Arc<Mutex<Self>> {
        Arc::new(Mutex::new(Inner { amplitude }))
    }
}

struct App {
    data: Arc<Mutex<BoundedVecDeque<[f64; 2]>>>,
    inner: Arc<Mutex<Inner>>
}

impl App {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let data: BoundedVecDeque<[f64; 2]> = BoundedVecDeque::new(1000);
        let mutex = Arc::new(Mutex::new(data));
        let inner = Inner::new(1.0);

        let app = App {
            data: mutex.clone(),
            inner: inner.clone()
        };

        let ctx = cc.egui_ctx.clone();

        thread::spawn(move || {
            let mut x = 0.0;
            loop {
                {
                    if let Ok(mut data_lock) = mutex.try_lock() {
                        if let Ok(lock) = inner.try_lock() {
                            let amplitude = lock.amplitude;
                            data_lock.push_back([x, amplitude * x.sin()]);
                        }
                    }
                }

                ctx.request_repaint();
                x += 0.01;

                thread::sleep(time::Duration::from_millis(10));
            }
        });

        app
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(&ctx, |ui| {
            {
                let x = &mut self.inner.lock().unwrap().amplitude;
                ui.add(egui::Slider::new(x, 0_f64..=100_f64).text("Amplitude"));
                let data: Vec<[f64; 2]> = self.data.lock().unwrap().clone().iter().cloned().collect();
                let points = PlotPoints::from_iter(data);
                Plot::new("Plot").view_aspect(2.0).show(ui, |plot_ui| plot_ui.line(Line::new(points)));
            }
        });
    }
}

use bounded_vec_deque::BoundedVecDeque;

fn main() {
    eframe::run_native("My app", eframe::NativeOptions::default(), Box::new(|cc| Box::new(App::new(cc))));
}
