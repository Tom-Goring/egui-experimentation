use chrono::{DateTime, Local, Utc, NaiveDateTime};
use eframe::egui;
use egui::{plot::{Line, Plot, PlotPoints}, TextStyle, ScrollArea};
use egui_dock::{DockArea, Style, Tree};

use core::time;
use std::{thread, sync::{Mutex, Arc}, collections::HashMap};
use std::time::{SystemTime, UNIX_EPOCH};

type Wrapped<T> = Arc<Mutex<T>>;
type SignalData = BoundedVecDeque<[f64; 2]>;
type DataMap = HashMap<String, SignalData>;

struct TabViewer {
    data: Wrapped<DataMap>
}

impl egui_dock::TabViewer for TabViewer {
    type Tab = String;

    fn ui(&mut self, ui: &mut egui::Ui, tab: &mut Self::Tab) {
        ui.label(format!("Content of {tab}"));
        match tab.as_str() {
            "tab1" => {
                let data = self.data.lock().unwrap();
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
            },
            "tab2" => {
                let text_style = TextStyle::Body;
                let row_height = ui.text_style_height(&text_style);
                let num_rows = 10_000;
                ScrollArea::vertical().id_source("Scroll Test").auto_shrink([false; 2]).show_rows(ui, row_height, num_rows, |ui, row_range| {
                    for row in row_range {
                        let text = format!("This is row {}/{}", row + 1, num_rows);
                        ui.label(text);
                    }
                });
            },
            _ => unreachable!()
        }
    }

    fn title(&mut self, tab: &mut Self::Tab) -> egui::WidgetText {
        (&*tab).into()
    }
}

struct App {
    data: Wrapped<DataMap>,
    tree: Tree<String>
}

impl App {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let data = Arc::new(Mutex::new(DataMap::new()));

        {
            let d = &mut data.lock().unwrap();
            d.insert("Hello".into(), BoundedVecDeque::new(1000));

        }

        let tree = Tree::new(vec!["tab1".to_owned(), "tab2".to_owned()]);

        let app = App {
            data: data.clone(),
            tree
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
        egui::CentralPanel::default().show(ctx, |_ui| {
            {
                DockArea::new(&mut self.tree).style(Style::from_egui(ctx.style().as_ref())).show(ctx, &mut TabViewer { data: self.data.clone() });
            }
        });
    }
}

use bounded_vec_deque::BoundedVecDeque;

fn main() {
    let opts = eframe::NativeOptions::default();
    eframe::run_native("My app", opts, Box::new(|cc| Box::new(App::new(cc))));
}
