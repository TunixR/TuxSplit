use crate::config::Config;
use crate::ui::timer::{TimerBody, TimerFooter, TimerHeader};

use core::time::Duration;
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::{Arc, RwLock};

use adw::prelude::*;
use adw::{self, AlertDialog, ApplicationWindow, Clamp, ToolbarView};
use glib::ControlFlow::Continue;
use gtk4::{
    gio, Align, Box as GtkBox, FileChooserDialog, FileFilter, Label, ListBox, Orientation::Vertical,
};

use livesplit_core::Timer;

// Timer layout for runs
pub struct TimerUI {
    timer: Arc<RwLock<Timer>>,
    config: Arc<RwLock<Config>>,
    header: Option<Rc<RefCell<TimerHeader>>>,
    body: Option<Rc<RefCell<TimerBody>>>,
    footer: Option<Rc<RefCell<TimerFooter>>>,
}

impl TimerUI {
    pub fn new(timer: Arc<RwLock<Timer>>, config: Arc<RwLock<Config>>) -> Self {
        Self {
            timer,
            config,
            header: None,
            body: None,
            footer: None,
        }
    }

    pub fn build_ui(&mut self, app: &adw::Application) -> adw::ApplicationWindow {
        let mut config_ref = self.config.write().unwrap();

        // --- Root Clamp ---
        let clamp = Clamp::builder().maximum_size(300).build();

        // === Outer VBox ===
        let livesplit_gtk = GtkBox::builder()
            .orientation(Vertical)
            .valign(Align::Center)
            .halign(Align::Center)
            .margin_top(24)
            .margin_bottom(24)
            .margin_start(24)
            .margin_end(24)
            .spacing(20)
            .build();

        // =====================
        // Component-based layout
        // =====================
        let header_comp = Rc::new(RefCell::new(TimerHeader::new(&self.timer.read().unwrap())));
        let body_comp = Rc::new(RefCell::new(TimerBody::new(
            &self.timer.read().unwrap(),
            &mut config_ref,
        )));
        let footer_comp = Rc::new(RefCell::new(TimerFooter::new(
            &self.timer.read().unwrap(),
            &mut config_ref,
            body_comp.borrow().list(),
        )));

        // =====================
        // Timeout: update children
        // =====================
        let timer_binding = self.timer.clone();
        let config_binding = self.config.clone();
        let header_binding = header_comp.clone();
        let body_binding = body_comp.clone();
        let footer_binding = footer_comp.clone();
        glib::timeout_add_local(Duration::from_millis(16), move || {
            let t = timer_binding.read().unwrap();
            let mut c = config_binding.write().unwrap();
            let mut h = header_binding.borrow_mut();
            let mut b = body_binding.borrow_mut();
            let mut f = footer_binding.borrow_mut();

            h.refresh(&t, &mut c);
            b.refresh(&t, &mut c);
            f.refresh(&t, &mut c);

            Continue
        });

        // =====================
        // Assemble everything
        // =====================
        self.header.replace(header_comp);
        self.body.replace(body_comp);
        self.footer.replace(footer_comp);

        livesplit_gtk.append(self.header.clone().unwrap().borrow().container());
        livesplit_gtk.append(self.body.clone().unwrap().borrow().container());
        livesplit_gtk.append(self.footer.clone().unwrap().borrow().container());

        clamp.set_child(Some(&livesplit_gtk));

        // Building the window
        let window = adw::ApplicationWindow::builder()
            .application(app)
            .title(
                Label::builder()
                    .label("TuxSplit")
                    .css_classes(["heading"])
                    .build()
                    .label(),
            )
            .resizable(false)
            .build();

        let view = ToolbarView::new();
        let header = self.build_main_header(&window);
        view.add_top_bar(&header);
        view.set_content(Some(&clamp));

        window.set_content(Some(&view));

        window
    }

    fn build_main_header(&self, parent: &ApplicationWindow) -> adw::HeaderBar {
        let header = adw::HeaderBar::builder()
            .title_widget(&Label::new(Some("TuxSplit")))
            .show_end_title_buttons(true)
            .build();

        // Hamburger menu with Load/Save Splits using a proper application menu
        let menu_button = gtk4::MenuButton::builder()
            .icon_name("open-menu-symbolic")
            .build();

        // Build a MenuModel and attach actions on the application (app.*)
        let menu = gio::Menu::new();

        let splits_section = gio::Menu::new();
        splits_section.append(Some("Load Splits"), Some("app.load-splits"));
        splits_section.append(Some("Save Splits"), Some("app.save-splits"));

        let settings_section = gio::Menu::new();
        settings_section.append(Some("Settings"), Some("app.settings"));
        settings_section.append(Some("Keybindings"), Some("app.keybindings"));

        let about_section = gio::Menu::new();
        about_section.append(Some("About"), Some("app.about"));

        menu.append_section(None, &splits_section);
        menu.append_section(None, &settings_section);
        menu.append_section(None, &about_section);
        menu_button.set_menu_model(Some(&menu));

        // Load Splits action
        let load_action = self.get_load_action(parent);

        // Save Splits action
        let save_action = self.get_save_action();

        // TODO: Config
        let settings_action = TimerUI::get_settings_action(parent);

        // Keybinds (For now only shows default keybinds)
        // TODO: Sync with config hotkeys
        let keybinds_action = TimerUI::get_keybinds_action(parent);

        // About action
        let about_action = TimerUI::get_about_action(parent);

        let group = gio::SimpleActionGroup::new();
        group.add_action(&load_action);
        group.add_action(&save_action);
        group.add_action(&settings_action);
        group.add_action(&keybinds_action);
        group.add_action(&about_action);

        menu_button.insert_action_group("app", Some(&group));

        header.pack_start(&menu_button);

        header
    }

    fn get_save_action(&self) -> gio::SimpleAction {
        let save_action = gio::SimpleAction::new("save-splits", None);
        let timer_for_save = self.timer.clone();
        let config_for_save = self.config.clone();
        save_action.connect_activate(move |_, _| {
            let t = timer_for_save.read().unwrap();
            let c = config_for_save.read().unwrap();
            c.save_splits(&t);
        });
        save_action
    }

    fn get_load_action(&self, parent: &ApplicationWindow) -> gio::SimpleAction {
        let load_action = gio::SimpleAction::new("load-splits", None);
        let timer_for_load = self.timer.clone();
        let config_for_load = self.config.clone();

        let parent_binding = parent.clone();

        load_action.connect_activate(move |_, _| {
            let file_chooser = FileChooserDialog::new(
                Some("Load Splits"),
                Some(&parent_binding),
                gtk4::FileChooserAction::Open,
                &[
                    ("Open", gtk4::ResponseType::Ok),
                    ("Cancel", gtk4::ResponseType::Cancel),
                ],
            );

            let lss_filter = FileFilter::new();
            let all_filter = FileFilter::new();
            lss_filter.set_name(Some("LiveSplit Splits (*.lss)"));
            all_filter.set_name(Some("All Files"));
            lss_filter.add_pattern("*.lss");
            all_filter.add_pattern("*");
            file_chooser.add_filter(&lss_filter);
            file_chooser.add_filter(&all_filter);

            let t_binding = timer_for_load.clone();
            let c_binding = config_for_load.clone();
            file_chooser.connect_response(move |dialog, response| {
                let mut c = c_binding.write().unwrap();
                let mut t = t_binding.write().unwrap();
                if response == gtk4::ResponseType::Ok {
                    if let Some(file) = dialog.file() {
                        if let Some(path) = file.path() {
                            c.set_splits_path(path);
                            if let Some(run) = c.parse_run() {
                                let _ = t.set_run(run);
                                c.configure_timer(&mut t);
                            }
                        }
                    }
                }
                dialog.destroy(); // This hides and closes the dialog window
            });

            file_chooser.set_modal(true);
            file_chooser.present();
        });
        load_action
    }

    fn get_keybinds_action(parent: &ApplicationWindow) -> gio::SimpleAction {
        let keybinds_action = gio::SimpleAction::new("keybindings", None);
        let parent_for_keybinds = parent.clone();
        keybinds_action.connect_activate(move |_, _| {
            let dialog = AlertDialog::builder()
                .heading("Keybindings")
                .body("Current keybinds are not modifiable yet.")
                .default_response("ok")
                .build();

            let keybinds_list = ListBox::new();
            keybinds_list.add_css_class("boxed-list");
            let keybinds = vec![
                ("Start / Split", "Numpad 1"),
                ("Skip Split", "Numpad 2"),
                ("Reset", "Numpad 3"),
                ("Previous Comparison", "Numpad 4"),
                ("Pause", "Numpad 5"),
                ("Next Comparison", "Numpad 6"),
                ("Undo", "Numpad 8"),
            ];
            for (action, key) in keybinds {
                let key_label = Label::new(Some(key));
                let row = adw::ActionRow::builder().title(action).build();
                row.add_suffix(&key_label);
                keybinds_list.append(&row);
            }

            dialog.set_extra_child(Some(&keybinds_list));

            dialog.add_response("ok", "Okay");
            dialog.present(Some(&parent_for_keybinds));
        });
        keybinds_action
    }

    fn get_settings_action(parent: &ApplicationWindow) -> gio::SimpleAction {
        let settings_action = gio::SimpleAction::new("settings", None);
        let parent_for_settings = parent.clone();
        settings_action.connect_activate(move |_, _| {
            let dialog = AlertDialog::builder()
                .heading("Settings")
                .body("This feature isn\u{2019}t available yet. Stay tuned!")
                .default_response("ok")
                .build();
            dialog.add_response("ok", "Okay");
            dialog.present(Some(&parent_for_settings));
        });
        settings_action
    }

    fn get_about_action(parent: &ApplicationWindow) -> gio::SimpleAction {
        let about_action = gio::SimpleAction::new("about", None);
        let parent_for_about = parent.clone();
        about_action.connect_activate(move |_, _| {
            let about_dialog = adw::AboutDialog::builder()
                .application_name("TuxSplit")
                .version("0.0.1")
                .comments("A GTK-based LiveSplit timer application.")
                .license_type(gtk4::License::MitX11)
                .website("https://github.com/AntonioRodriguezRuiz/tuxsplit")
                .build();
            about_dialog.present(Some(&parent_for_about));
        });
        about_action
    }
}
