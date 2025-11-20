pub mod body;
pub mod footer;
pub mod header;

use crate::ui::timer::body::TimerBody;
use crate::ui::timer::footer::TimerFooter;
use crate::ui::timer::header::TimerHeader;

use std::cell::RefCell;
use std::rc::Rc;

use core::time::Duration;

use adw::Clamp;
use adw::prelude::*;
use gtk4::{Align, Box as GtkBox, Orientation::Vertical};

use crate::context::TuxSplitContext;

pub struct TuxSplitTimer {
    clamp: Clamp,
    header: Rc<RefCell<TimerHeader>>,
    body: Rc<RefCell<TimerBody>>,
    footer: Rc<RefCell<TimerFooter>>,
    refresh_source: Option<glib::SourceId>,
}

impl TuxSplitTimer {
    /// Create the timer widget (header/body/footer composed) but does NOT start refresh loop.
    pub fn new() -> Self {
        let clamp = Clamp::builder().maximum_size(900).build();

        let container = GtkBox::builder()
            .orientation(Vertical)
            .valign(Align::Center)
            .halign(Align::Fill)
            .hexpand(true)
            .margin_top(24)
            .margin_bottom(24)
            .margin_start(24)
            .margin_end(24)
            .spacing(20)
            .build();

        let ctx = TuxSplitContext::get_instance();
        let timer_arc = ctx.timer();
        let timer_read = timer_arc.read().unwrap();
        let header = Rc::new(RefCell::new(TimerHeader::new(&timer_read)));

        let cfg = ctx.config();
        let body = Rc::new(RefCell::new(TimerBody::new(&timer_read, &cfg)));
        let footer = Rc::new(RefCell::new(TimerFooter::new(
            &timer_read,
            &cfg,
            body.borrow().list(),
            body.borrow().last_segment_list(),
        )));
        drop(timer_read);

        container.append(header.borrow().container());
        container.append(body.borrow().container());
        container.append(footer.borrow().container());

        clamp.set_child(Some(&container));

        {
            // Connect global run-changed to force a rebuild of timer UI.
            let body_binding = body.clone();
            let footer_binding = footer.clone();
            TuxSplitContext::get_instance().connect_local("run-changed", false, move |_| {
                let ctx = TuxSplitContext::get_instance();
                let t = {
                    let shared = ctx.timer();
                    shared.read().unwrap().clone()
                };
                let c = ctx.config();
                body_binding.borrow_mut().refresh(&t, &c, true);
                footer_binding.borrow_mut().refresh(&t, &c);
                None
            });
        }

        Self {
            clamp,
            header,
            body,
            footer,
            refresh_source: None,
        }
    }

    pub fn clamped(&self) -> &Clamp {
        &self.clamp
    }

    pub fn start_refresh_loop(&mut self) {
        if self.refresh_source.is_some() {
            return; // Already running
        }

        let header_binding = self.header.clone();
        let body_binding = self.body.clone();
        let footer_binding = self.footer.clone();

        let source_id = glib::timeout_add_local(Duration::from_millis(16), move || {
            let ctx = TuxSplitContext::get_instance();
            let t = {
                let shared = ctx.timer();
                shared.read().unwrap().clone()
            };

            let c = ctx.config();
            header_binding.borrow_mut().refresh(&t);
            body_binding.borrow_mut().refresh(&t, &c, false);
            footer_binding.borrow_mut().refresh(&t, &c);

            glib::ControlFlow::Continue
        });

        self.refresh_source = Some(source_id);
    }

    pub fn stop_refresh_loop(&mut self) {
        if let Some(id) = self.refresh_source.take() {
            id.remove();
        }
    }
}
