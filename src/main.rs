use std::{time::{Instant, Duration}, rc::Rc, cell::RefCell, ops::DerefMut};

use eframe::NativeOptions;
use poll_promise::Promise;
use smol::{LocalExecutor, future, Timer};
use tracing::info;

macro_rules! enclose {
    ( ($( $x:ident ),*) $y:expr ) => {
        {
            $(let $x = $x.clone();)*
            $y
        }
    };
}

#[derive(Default)]
struct App {
    message: Rc<RefCell<String>>,
    _started_at: Option<Instant>,
    futures: Vec<Promise<()>>,
    string: String
}

impl App {

}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        for future in self.futures.iter() {
            future.ready();
        }

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.label(self.message.clone().borrow().as_str());
            let mut message = self.message.borrow_mut();
            egui::TextEdit::singleline(message.deref_mut()).show(ui);
            if ui.button("Start timer").clicked() {
                self.futures.push(Promise::spawn_local(enclose! { (self.message) async move {
                    Timer::after(Duration::from_secs(1)).await;
                    message.replace("hello".into());
                }}));
            }   
        });
    }
}

fn main() {
    tracing_subscriber::fmt::init();
    
    let local = LocalExecutor::new();

    future::block_on(local.run(async {
        eframe::run_native("Test Async App", NativeOptions::default(), Box::new(|_cc| Box::new(App { ..Default::default() })));
    }));


    // future::block_on(local.run(async {
    // }));
    // local.run_until(async move {
    //     let promise = Promise::spawn_local(async {
    //         tokio::time::sleep(Duration::from_secs(1)).await;
    //         info!("Hello");
    //     });
    //
    //     loop {
    //         if promise.ready().is_some() {
    //             info!("Done");
    //             break;
    //         }
    //         else {
    //             info!("Waiting");
    //         }
    //     }
    //
    // }).await;

}
