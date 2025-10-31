// This file defines the user interfaces for the application

use super::super::config::Config;

use std::sync::{Arc, RwLock};
use std::time::Duration;

use adw::prelude::*;
use adw::ApplicationWindow;
use adw::{self, Clamp};
use glib::ControlFlow::Continue;
use gtk4::Orientation::{Horizontal, Vertical};
use gtk4::{Align, Box as GtkBox, CenterBox, Label, ListBox};

use livesplit_core::{Run, Segment, Timer, TimerPhase};

use tracing::debug;

// Main screen for load / create splits
pub struct MainUI {}

// Timer layout for runs
pub struct TimerUI {
    timer: Arc<RwLock<Timer>>,
    config: Arc<RwLock<Config>>,
}

// Splits editor/Creator
pub struct EditorUI {}

pub struct SettingsUI {}

pub struct AboutUI {}

pub struct HelpUI {}

impl TimerUI {
    pub fn new(timer: Arc<RwLock<Timer>>, config: Arc<RwLock<Config>>) -> Self {
        Self { timer, config }
    }

    pub fn build_ui(&self) -> adw::Clamp {
        // --- Root Clamp ---
        let clamp = Clamp::builder().maximum_size(300).build();

        // === Outer VBox ===
        let livesplit_gtk = GtkBox::builder()
            .orientation(Vertical)
            .valign(Align::Center)
            .halign(Align::Center)
            .width_request(300)
            .margin_top(24)
            .margin_bottom(24)
            .margin_start(24)
            .margin_end(24)
            .spacing(20)
            .build();

        // =====================
        // Run Info Section
        // =====================
        let run_info = TimerUI::build_run_info(&self.timer.read().unwrap());

        //
        // Splits List
        // =====================
        let splits = ListBox::new();
        splits.add_css_class("boxed-list");
        let splits_rows = TimerUI::build_splits_list(&self.timer.read().unwrap());
        for row in splits_rows {
            splits.append(&row);
        }

        // =====================
        // Current Split + Timer
        // =====================
        let center_box = CenterBox::builder()
            .orientation(Horizontal)
            .margin_start(18)
            .margin_end(18)
            .build();
        center_box.set_start_widget(Some(&TimerUI::build_center_box_current_split_info(
            &self.timer.read().unwrap(),
            &self.config.read().unwrap(),
        )));
        center_box.set_end_widget(Some(&TimerUI::build_center_box_timer(
            &self.timer.read().unwrap(),
            &self.config.read().unwrap(),
        )));

        let splits_binding = splits.clone();
        let center_box_binding = center_box.clone();

        let timer_binding = self.timer.clone();
        let config_binding = self.config.clone();

        glib::timeout_add_local(Duration::from_millis(16), move || {
            let t = timer_binding.read().unwrap();
            let c = config_binding.read().unwrap();
            // =====================
            // Splits List
            // =====================
            // Remove all existing rows
            for (index, _) in t.run().segments().iter().enumerate() {
                if let Some(row) = splits_binding.row_at_index(0) {
                    splits_binding.remove(&row);
                }
            }
            // Now rebuild
            let splits_rows = TimerUI::build_splits_list(&t);
            for row in splits_rows {
                splits_binding.append(&row);
            }

            // =====================
            // Current Split + Timer
            // =====================
            center_box_binding
                .set_start_widget(Some(&TimerUI::build_center_box_current_split_info(&t, &c)));
            center_box_binding.set_end_widget(Some(&TimerUI::build_center_box_timer(&t, &c)));

            // =====================
            // Assemble everything
            // =====================
            Continue
        });

        // =====================
        // Assemble everything
        // =====================
        livesplit_gtk.append(&run_info);
        livesplit_gtk.append(&splits);
        livesplit_gtk.append(&center_box);

        clamp.set_child(Some(&livesplit_gtk));

        clamp
    }
}

impl TimerUI {
    fn build_run_info(timer: &Timer) -> GtkBox {
        let run_info = GtkBox::builder()
            .orientation(Vertical)
            .halign(Align::Center)
            .build();

        let run_name = Label::builder().label(timer.run().game_name()).build();
        run_name.add_css_class("title-2");
        debug!("Run Name: {}", run_name.label());

        let category = Label::builder().label(timer.run().category_name()).build();
        category.add_css_class("heading");
        debug!("Category: {}", category.label());

        run_info.append(&run_name);
        run_info.append(&category);
        run_info
    }

    fn build_splits_list(timer: &Timer) -> Vec<adw::ActionRow> {
        let mut rows = Vec::new();

        let segments = timer.run().segments();
        let opt_current_segment_index = timer.current_split_index();

        for (index, segment) in segments.iter().enumerate() {
            let title = segment.name();
            let mut value = String::from("--");

            if let Some(current_segment_index) = opt_current_segment_index {
                if current_segment_index > index {
                    value = format!(
                        "{:.2}",
                        segment
                            .split_time() // TODO: Implement custom css based on if ahead or behind
                            .real_time
                            .unwrap_or_default()
                            .to_duration()
                    ); // TODO: Allow for time instead of comparison | Allow for gametime/realtime comparison
                }
                if current_segment_index == index {
                    value = String::from("WIP") // TODO: Allow for time instead of comparison | Allow for gametime/realtime comparison
                }
            }

            let classes = if index == segments.len() - 1 {
                &["finalsplit"][..]
            } else {
                &["split"][..]
            };

            rows.push(Self::make_split_row(title, &value, classes));
        }

        rows
    }

    fn build_center_box_current_split_info(timer: &Timer, config: &Config) -> GtkBox {
        // Left side: current split info
        let current_split = GtkBox::builder().orientation(Vertical).build();

        // Best
        let best_box = GtkBox::builder()
            .orientation(Horizontal)
            .margin_top(6)
            .spacing(2)
            .halign(Align::Start)
            .build();
        let best_label = Label::builder().label("Best:").build();
        best_label.add_css_class("caption-heading");

        let segments = timer.run().segments();
        let best_comparison_split = timer.current_split().unwrap_or(segments.get(0).unwrap());
        let best_comparison_time = best_comparison_split
            .best_segment_time()
            .real_time
            .unwrap_or_default();

        let best_minutes = best_comparison_time.total_seconds() as i32 / 60 % 60;
        let best_seconds = best_comparison_time.total_seconds() as i32 % 60;
        let best_milliseconds = best_comparison_time.total_milliseconds() as i32 % 1000;
        let best_value = Label::builder()
            .label(format!(
                "{}:{:02}.{:02}",
                best_minutes, best_seconds, best_milliseconds
            ))
            .build();
        best_value.add_css_class("caption");
        best_value.add_css_class("timer");
        best_box.append(&best_label);
        best_box.append(&best_value);

        // Comparison
        let comparison_box = GtkBox::builder()
            .orientation(Horizontal)
            .spacing(2)
            .halign(Align::Start)
            .build();
        let comparison_label = Label::builder() // TODO: Map comparisons to simpler string representations
            .label(format!(
                "{}:",
                config
                    .general
                    .comparison
                    .as_ref()
                    .unwrap_or(&String::from("PB"))
            ))
            .build();
        comparison_label.add_css_class("caption-heading");

        let comparison_time = timer
            .current_split()
            .unwrap_or(segments.get(0).unwrap())
            .comparison(
                config
                    .general
                    .comparison
                    .as_ref()
                    .unwrap_or(&String::from("")),
            ) // TODO: Implement custom css based on if ahead or behind
            .real_time
            .unwrap_or_default();

        let comparison_minutes = comparison_time.total_seconds() as i32 / 60 % 60;
        let comparison_seconds = comparison_time.total_seconds() as i32 % 60;
        let comparison_milliseconds = comparison_time.total_milliseconds() as i32 % 1000;
        let comparison_value = Label::builder()
            .label(format!(
                "{}:{:02}.{:02}",
                comparison_minutes, comparison_seconds, comparison_milliseconds
            ))
            .build();

        comparison_value.add_css_class("caption");
        comparison_value.add_css_class("timer");
        comparison_box.append(&comparison_label);
        comparison_box.append(&comparison_value);

        current_split.append(&best_box);
        current_split.append(&comparison_box);

        current_split
    }

    fn build_center_box_timer(timer: &Timer, config: &Config) -> GtkBox {
        // Right side: timer display
        let timer_box = GtkBox::new(Horizontal, 0);
        timer_box.add_css_class("timer");
        timer_box.add_css_class("greensplit");

        let time = timer.current_attempt_duration();
        let minutes = time.total_seconds() as i32 / 60 % 60;
        let seconds = time.total_seconds() as i32 % 60;
        let hour_minutes_seconds_timer = Label::builder()
            .label(format!("{:02}:{:02}.", minutes, seconds))
            .build();
        hour_minutes_seconds_timer.add_css_class("bigtimer");

        let milliseconds = time.total_milliseconds() as i32 % 100;
        let milis_timer = Label::builder()
            .label(format!("{:02}", milliseconds))
            .margin_top(14)
            .build();
        milis_timer.add_css_class("smalltimer");

        timer_box.append(&hour_minutes_seconds_timer);
        timer_box.append(&milis_timer);

        timer_box
    }

    fn make_split_row(title: &str, value: &str, classes: &[&str]) -> adw::ActionRow {
        let row = adw::ActionRow::builder().title(title).build();
        let label = Label::builder()
            .label(value)
            .halign(Align::Center)
            .valign(Align::Center)
            .build();
        label.add_css_class("timer");
        for cls in classes {
            label.add_css_class(cls);
        }
        row.add_suffix(&label);
        row
    }
}
