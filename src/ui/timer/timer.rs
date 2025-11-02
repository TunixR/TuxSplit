use crate::config::Config;
use crate::ui::timer::{data_model, widgets};

use std::sync::{Arc, RwLock};
use std::time::Duration;

use adw::prelude::*;
use adw::{self, ApplicationWindow, Clamp, ToolbarView};
use glib::ControlFlow::Continue;
use gtk4::{
    Align, Box as GtkBox, CenterBox, Label, ListBox,
    Orientation::{Horizontal, Vertical},
    SelectionMode,
};

use livesplit_core::{Timer, TimerPhase};

use tracing::debug;

// Timer layout for runs
pub struct TimerUI {
    timer: Arc<RwLock<Timer>>,
    config: Arc<RwLock<Config>>,
}

impl TimerUI {
    pub fn new(timer: Arc<RwLock<Timer>>, config: Arc<RwLock<Config>>) -> Self {
        Self { timer, config }
    }

    pub fn build_ui(&self) -> ToolbarView {
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
        let segments_list = ListBox::new();
        segments_list.set_selection_mode(SelectionMode::Single);
        segments_list.add_css_class("boxed-list");
        let mut config_ref = self.config.write().unwrap();
        let segments_rows =
            TimerUI::build_splits_list(&self.timer.read().unwrap(), &mut config_ref);
        for row in segments_rows {
            segments_list.append(&row);
        }
        segments_list.unselect_all();

        // =====================
        // Current Split + Timer
        // =====================
        let center_box = CenterBox::builder()
            .orientation(Horizontal)
            .margin_start(18)
            .margin_end(18)
            .build();
        center_box.set_start_widget(Some(&TimerUI::build_center_box_selected_segment_info(
            &self.timer.read().unwrap(),
            &mut config_ref,
            &segments_list,
        )));
        center_box.set_end_widget(Some(&TimerUI::build_center_box_timer(
            &self.timer.read().unwrap(),
            &mut config_ref,
        )));

        let segments_binding = segments_list.clone();
        let center_box_binding = center_box.clone();

        let mut rendered_comparison = self.timer.read().unwrap().current_comparison().to_string();
        let mut rendered_phase = self.timer.read().unwrap().current_phase();
        let mut render_all_segments = true;

        let timer_binding = self.timer.clone();
        let config_binding = self.config.clone();

        glib::timeout_add_local(Duration::from_millis(16), move || {
            let t = timer_binding.read().unwrap();
            let mut c = config_binding.write().unwrap();

            render_all_segments = (rendered_comparison != t.current_comparison().to_string())
                || (rendered_phase != t.current_phase());
            rendered_comparison = t.current_comparison().to_string();
            rendered_phase = t.current_phase();

            // =====================
            // Splits List
            // =====================
            // Remove all existing rows
            if render_all_segments {
                render_all_segments = false;
                segments_binding.set_selection_mode(SelectionMode::Single);

                let mut selected_index: Option<i32> = None;

                // REBUILD ONCE
                for (index, _) in t.run().segments().iter().enumerate() {
                    if let Some(row) = segments_binding.row_at_index(0) {
                        if row.is_selected() {
                            selected_index = Some(index as i32);
                        }
                        segments_binding.remove(&row);
                    }
                }
                let splits_rows = TimerUI::build_splits_list(&t, &mut c);
                for row in splits_rows {
                    segments_binding.append(&row);
                }

                if t.current_phase().is_ended() {
                    segments_binding.select_row(
                        segments_binding
                            .row_at_index(
                                selected_index
                                    .unwrap_or(t.run().segments().len().saturating_sub(1) as i32),
                            )
                            .as_ref(),
                    );
                } else {
                    segments_binding.unselect_all();
                }
            } else if t.current_phase().is_running() {
                render_all_segments = true;
                segments_binding.set_selection_mode(SelectionMode::None);

                let opt_current_segment_index = t.current_split_index().unwrap_or(0);
                let segments = t.run().segments();

                for (index, _) in segments.iter().enumerate() {
                    if let Some(row) = segments_binding.row_at_index(index as i32) {
                        // Set rows as not selectable to avoid interaction during update
                        if index == opt_current_segment_index
                            || index == opt_current_segment_index.saturating_sub(1)
                            || index == opt_current_segment_index.saturating_add(1)
                        {
                            segments_binding.remove(&row);
                            let row = widgets::split_row(&data_model::compute_segment_row(
                                &t,
                                &mut c,
                                Some(opt_current_segment_index),
                                index,
                                &segments[index],
                            ));
                            segments_binding.insert(&row, index as i32);
                        }
                    }
                }
            }

            // =====================
            // Current Split + Timer
            // =====================
            center_box_binding.set_start_widget(Some(
                &TimerUI::build_center_box_selected_segment_info(&t, &mut c, &segments_binding),
            ));
            center_box_binding.set_end_widget(Some(&TimerUI::build_center_box_timer(&t, &mut c)));

            Continue
        });

        // =====================
        // Assemble everything
        // =====================
        livesplit_gtk.append(&run_info);
        livesplit_gtk.append(&segments_list);
        livesplit_gtk.append(&center_box);

        clamp.set_child(Some(&livesplit_gtk));

        // Building the window
        let view = ToolbarView::new();
        let header = adw::HeaderBar::builder()
            .title_widget(&Label::new(Some("LiveSplit GTK")))
            .show_end_title_buttons(true)
            .build();
        view.add_top_bar(&header);
        view.set_content(Some(&clamp));

        view
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

    fn build_splits_list(timer: &Timer, config: &mut Config) -> Vec<adw::ActionRow> {
        data_model::compute_split_rows(timer, config)
            .into_iter()
            .map(|d| {
                let row = widgets::split_row(&d);
                row
            })
            .collect()
    }

    fn build_center_box_selected_segment_info(
        timer: &Timer,
        config: &mut Config,
        segments_list: &ListBox,
    ) -> GtkBox {
        let data = data_model::compute_selected_segment_info(timer, config, segments_list);
        widgets::build_selected_segment_info_box(&data)
    }

    fn build_center_box_timer(timer: &Timer, config: &mut Config) -> GtkBox {
        widgets::build_timer_box(timer, config)
    }
}
