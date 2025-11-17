use std::sync::OnceLock;

use glib::prelude::*;
use glib::subclass::prelude::*;
use glib::subclass::signal::Signal;

use gtk4::{ActionBar, Button, prelude::*};

mod imp {
    use super::*;

    // Plain GObject that owns a gtk4::ActionBar internally
    pub struct SaveCancelActionBar {
        pub action_bar: ActionBar,
    }

    impl Default for SaveCancelActionBar {
        fn default() -> Self {
            Self {
                action_bar: ActionBar::new(),
            }
        }
    }

    #[glib::object_subclass]
    impl ObjectSubclass for SaveCancelActionBar {
        const NAME: &'static str = "SaveCancelActionBar";
        type Type = super::SaveCancelActionBar;
        type ParentType = glib::Object;
    }

    impl ObjectImpl for SaveCancelActionBar {
        fn constructed(&self) {
            self.parent_constructed();

            let obj = self.obj();

            // Build buttons
            let save_button = Button::with_label("Save");
            save_button.add_css_class("suggested-action");

            let cancel_button = Button::with_label("Cancel");

            // Emit the same signal with a parameter indicating which one was pressed
            {
                let obj_weak = obj.downgrade();
                save_button.connect_clicked(move |_| {
                    if let Some(obj) = obj_weak.upgrade() {
                        obj.emit_by_name::<()>("save-cancel-action", &[&"save"]);
                    }
                });
            }
            {
                let obj_weak = obj.downgrade();
                cancel_button.connect_clicked(move |_| {
                    if let Some(obj) = obj_weak.upgrade() {
                        obj.emit_by_name::<()>("save-cancel-action", &[&"cancel"]);
                    }
                });
            }

            // Pack into the internal action bar
            self.action_bar.pack_start(&cancel_button);
            self.action_bar.pack_end(&save_button);
        }

        fn signals() -> &'static [Signal] {
            static SIGNALS: OnceLock<Vec<Signal>> = OnceLock::new();
            SIGNALS.get_or_init(|| {
                vec![
                    // Emitted when either Save or Cancel is clicked.
                    // First (and only) parameter is a string: "save" or "cancel".
                    Signal::builder("save-cancel-action")
                        .param_types([String::static_type()])
                        .action()
                        .build(),
                ]
            })
        }
    }
}

glib::wrapper! {
    pub struct SaveCancelActionBar(ObjectSubclass<imp::SaveCancelActionBar>);
}

impl SaveCancelActionBar {
    pub fn new() -> Self {
        glib::Object::builder().build()
    }

    // Expose the internal ActionBar to be packed into the UI
    pub fn widget(&self) -> ActionBar {
        self.imp().action_bar.clone()
    }
}
