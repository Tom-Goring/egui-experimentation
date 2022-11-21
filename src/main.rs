use chrono::{DateTime, Local, Utc, NaiveDateTime};
use eframe::egui;
use egui::plot::{Line, Plot, PlotPoints};

use core::time;
use std::{thread, sync::{Mutex, Arc}, collections::HashMap};
use std::time::{SystemTime, UNIX_EPOCH};

type Wrapped<T> = Arc<Mutex<T>>;
type SignalData = BoundedVecDeque<[f64; 2]>;
type DataMap = HashMap<String, SignalData>;

struct App {
    data: Wrapped<DataMap>
}

impl App {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let data = Arc::new(Mutex::new(DataMap::new()));

        {
            let d = &mut data.lock().unwrap();
            d.insert("Hello".into(), BoundedVecDeque::new(1000));

        }

        let app = App {
            data: data.clone()
        };

        let ctx = cc.egui_ctx.clone();

        thread::spawn(move || {
            let mut x: f64 = 0.0;
            loop {
                {
                    if let Ok(mut data) = data.try_lock() {
                        for (_, v) in data.iter_mut() {
                            let now_as_millis = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis();
                            let n = now_as_millis as f64;
                            v.push_back([n, x.sin()]);
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
        egui::CentralPanel::default().show(ctx, |ui| {
            {
                let data = self.data.lock().unwrap();
                egui::ScrollArea::vertical().show(ui, |ui| {
                    for (k, v) in data.iter() {
                        ui.add(egui::Label::new(k));
                        let data: Vec<[f64; 2]> = v.clone().iter().cloned().collect();
                        let points = PlotPoints::from_iter(data);
                        Plot::new(k).view_aspect(2.0).x_axis_formatter(|x, _range| { 
                            let datetime = NaiveDateTime::from_timestamp_millis(x as i64).unwrap();
                            let timestamp = DateTime::<Local>::from(DateTime::<Utc>::from_utc(datetime, Utc));
                            format!("{}", timestamp.format("%H:%M:%S"))
                        }).label_formatter(|_, xy| {
                            let datetime = NaiveDateTime::from_timestamp_millis(xy.x as i64).unwrap();
                            let timestamp = DateTime::<Local>::from(DateTime::<Utc>::from_utc(datetime, Utc));
                            format!("x: {}\ny: {}", timestamp.format("%H:%M:%S"), xy.y) 
                        }).show(ui, |plot_ui| plot_ui.line(Line::new(points)));
                    }
                });
                // let x = &mut self.inner.lock().unwrap().amplitude;
                // ui.add(egui::Slider::new(x, 0_f64..=100_f64).text("Amplitude"));
            }
        });
    }
}

use bounded_vec_deque::BoundedVecDeque;

fn main() {
    let opts = eframe::NativeOptions::default();
    eframe::run_native("My app", opts, Box::new(|cc| Box::new(App::new(cc))));
}
