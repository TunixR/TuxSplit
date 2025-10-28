// mod api;

use std::cell::RefCell;
use std::rc::Rc;
use std::sync::{Arc, RwLock};
use std::time::Duration;

// use api::api::{create, reset, split, start};

use livesplit_core::{Run, Segment, Timer, TimerPhase};
use tracing_subscriber;

use glib::ControlFlow::Continue;
use gtk4::prelude::*;
use gtk4::{Application, ApplicationWindow, Box as GtkBox, Button, Label, Orientation};

fn main() {
    let app = Application::builder()
        .application_id("org.UnixSplit.unixplit-beta")
        .build();

    let app_state = Rc::new(RefCell::new(UnixSplit::new()));

    app.connect_activate(move |app| {
        app_state.borrow_mut().build_ui(app);
    });
    app.run();
}

#[derive(Clone, Debug)]
pub struct UnixSplit {
    pub timer: Rc<RefCell<Timer>>,
}

impl UnixSplit {
    pub fn new() -> Self {
        let mut run = Run::new();
        run.push_segment(Segment::new(""));

        let timer = Timer::new(run).expect("");

        Self {
            timer: Rc::new(RefCell::new(timer)),
        }
    }

    fn build_ui(&mut self, app: &Application) {
        let window = ApplicationWindow::builder()
            .application(app)
            .title("LiveSplit GTK Starter")
            .default_width(400)
            .default_height(120)
            .build();

        let vbox = GtkBox::new(Orientation::Vertical, 8);
        vbox.set_margin_top(12);
        vbox.set_margin_bottom(12);
        vbox.set_margin_start(12);
        vbox.set_margin_end(12);

        let time_label = Label::new(Some("Not running"));
        time_label.set_wrap(true);
        vbox.append(&time_label);

        let hbox = GtkBox::new(Orientation::Horizontal, 6);

        let start_button = Button::with_label("Start");
        let split_button = Button::with_label("Split");
        let reset_button = Button::with_label("Reset");

        hbox.append(&start_button);
        hbox.append(&split_button);
        hbox.append(&reset_button);

        vbox.append(&hbox);

        window.set_child(Some(&vbox));

        // Clone handles for closures
        let timer_for_start = self.timer.clone();
        start_button.connect_clicked(move |_| {
            let mut t = timer_for_start.borrow_mut();
            if t.current_phase() == TimerPhase::NotRunning {
                t.start();
            }
        });

        let timer_for_split = self.timer.clone();
        split_button.connect_clicked(move |_| {
            let mut t = timer_for_split.borrow_mut();
            if t.current_phase() == TimerPhase::Running {
                t.split();
            }
        });

        let timer_for_reset = self.timer.clone();
        reset_button.connect_clicked(move |_| {
            let mut t = timer_for_reset.borrow_mut();
            t.reset(true);
        });

        let time_label_updater = time_label.clone();
        let timer_for_loop = self.timer.clone();

        // GLib timeout to update UI on the main thread.
        glib::timeout_add_local(Duration::from_millis(16), move || {
            let t = timer_for_loop.borrow_mut();
            match t.current_phase() {
                TimerPhase::NotRunning => {
                    time_label_updater.set_text("Not running");
                }
                TimerPhase::Running => {
                    let time = t.current_attempt_duration();
                    let hours = time.total_seconds() as i32 / 3600;
                    let minutes = time.total_seconds() as i32 / 60 % 60;
                    let seconds = time.total_seconds() as i32 % 60;
                    let milliseconds = time.total_milliseconds() as i32 % 1000;
                    let s = format!(
                        "{:02}:{:02}:{:02}.{:03}",
                        hours, minutes, seconds, milliseconds
                    );
                    time_label_updater.set_text(&s);
                }
                TimerPhase::Ended => {
                    time_label_updater.set_text("Run ended");
                }
                TimerPhase::Paused => {
                    time_label_updater.set_text("Paused");
                }
            }

            Continue
        });

        window.show();
    }
}
