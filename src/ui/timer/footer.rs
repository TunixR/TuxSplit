use crate::config::Config;
use crate::formatters::label::format_label;

use glib;
use gtk4::prelude::{BoxExt as _, WidgetExt as _, *};
use gtk4::{
    Align, Box as GtkBox, CenterBox, Label, ListBox, Orientation::Horizontal, Orientation::Vertical,
};

use livesplit_core::{Timer, TimerPhase};

pub struct TimerFooter {
    container: CenterBox,
    segment_comparison: SegmentComparison,
    running_timer: RunningTimer,
}

impl TimerFooter {
    pub fn new(timer: &Timer, config: &mut Config, list_for_selection: &ListBox) -> Self {
        let container = CenterBox::builder()
            .orientation(Horizontal)
            .width_request(300)
            .build();

        let segment_comparison = SegmentComparison::new(timer, config, list_for_selection);
        let running_timer = RunningTimer::new(timer, config);

        container.set_start_widget(Some(segment_comparison.container()));
        container.set_end_widget(Some(running_timer.container()));

        Self {
            container,
            segment_comparison,
            running_timer,
        }
    }

    pub fn container(&self) -> &CenterBox {
        &self.container
    }

    pub fn refresh(&mut self, timer: &Timer, config: &mut Config) {
        self.segment_comparison.update(timer, config);
        self.running_timer.update(timer, config);

        self.container
            .set_start_widget(Some(self.segment_comparison.container()));
        self.container
            .set_end_widget(Some(self.running_timer.container()));
    }
}

/// Left pane in the footer:
/// - Best: <best split value>
/// - <Comparison Label>: <per-segment comparison value>
pub struct SegmentComparison {
    wrapper: GtkBox,
    segments_list_ref: glib::WeakRef<ListBox>, // The segment list can be removed without panic. This will just return None in that case
    best_value: Label,
    comparison_label: Label,
    comparison_value: Label,
}

impl SegmentComparison {
    pub fn new(timer: &Timer, config: &mut Config, list_for_selection: &ListBox) -> Self {
        let build = GtkBox::builder().orientation(Vertical).build();
        let wrapper = build;

        let vbox = GtkBox::builder().orientation(Vertical).build();

        let (best_box, best_value) = SegmentComparison::build_best();

        let (comparison_box, comparison_label, comparison_value) =
            SegmentComparison::build_comparison();

        vbox.append(&best_box);
        vbox.append(&comparison_box);
        wrapper.append(&vbox);

        let mut this = Self {
            wrapper,
            segments_list_ref: glib::WeakRef::new(),
            best_value,
            comparison_label,
            comparison_value,
        };
        this.set_segments_list(list_for_selection);
        this.rebuild(timer, config);
        this
    }
    pub fn container(&self) -> &GtkBox {
        &self.wrapper
    }

    pub fn set_segments_list(&mut self, list_for_selection: &ListBox) {
        self.segments_list_ref.set(Some(list_for_selection));
    }

    pub fn update(&mut self, timer: &Timer, config: &mut Config) {
        self.rebuild(timer, config);
    }

    fn rebuild(&mut self, timer: &Timer, config: &mut Config) {
        // Compute which segment to display
        if let Some(list) = self.segments_list_ref.upgrade() {
            let segments = timer.run().segments();

            let selected_index = if timer.current_phase().is_running() {
                timer.current_split_index().unwrap_or(0)
            } else {
                list.selected_row().map_or(0, |row| row.index() as usize)
            }
            .min(segments.len().saturating_sub(1));

            let segment = &segments[selected_index];

            // Previous segment's comparison time (under current timing method)
            let previous_comparison_time = if selected_index > 0 {
                segments[selected_index - 1]
                    .comparison_timing_method(
                        timer.current_comparison(),
                        timer.current_timing_method(),
                    )
                    .unwrap_or_default()
                    .to_duration()
            } else {
                time::Duration::ZERO
            };

            // Build values
            let best_value_text = config
                .format
                .split
                .format_split_time(&segment.best_segment_time(), timer.current_timing_method());

            let comparison_label_text = format!("{}:", format_label(timer.current_comparison()));

            let comparison_value_text = config.format.segment.format_segment_time(
                &segment
                    .comparison_timing_method(
                        timer.current_comparison(),
                        timer.current_timing_method(),
                    )
                    .unwrap_or_default()
                    .to_duration()
                    .checked_sub(previous_comparison_time)
                    .unwrap_or_default()
                    .abs(),
            );

            // Update stored labels in place
            if self.best_value.label().as_str() != best_value_text {
                self.best_value.set_label(&best_value_text);
            }
            if self.comparison_label.label().as_str() != comparison_label_text {
                self.comparison_label.set_label(&comparison_label_text);
            }
            if self.comparison_value.label().as_str() != comparison_value_text {
                self.comparison_value.set_label(&comparison_value_text);
            }
        } else {
            // No list available; show placeholders
            if self.best_value.label().as_str() != "--" {
                self.best_value.set_label("--");
            }
            if self.comparison_label.label().as_str() != "PB:" {
                self.comparison_label.set_label("PB:");
            }
            if self.comparison_value.label().as_str() != "--" {
                self.comparison_value.set_label("--");
            }
        }
    }

    fn build_comparison() -> (GtkBox, Label, Label) {
        let comparison_box = GtkBox::builder()
            .orientation(Horizontal)
            .spacing(2)
            .halign(Align::Start)
            .build();

        let comparison_label = Label::builder().label("PB:").build();
        comparison_label.add_css_class("caption-heading");

        let comparison_value = Label::builder().label("--").build();
        comparison_value.add_css_class("caption");
        comparison_value.add_css_class("timer");

        comparison_box.append(&comparison_label);
        comparison_box.append(&comparison_value);
        (comparison_box, comparison_label, comparison_value)
    }

    fn build_best() -> (GtkBox, Label) {
        let best_box = GtkBox::builder()
            .orientation(Horizontal)
            .margin_top(6)
            .spacing(2)
            .halign(Align::Start)
            .build();
        let best_label = Label::builder().label("Best:").build();
        best_label.add_css_class("caption-heading");

        let best_value = Label::builder().label("--").build();
        best_value.add_css_class("caption");
        best_value.add_css_class("timer");

        best_box.append(&best_label);
        best_box.append(&best_value);
        (best_box, best_value)
    }
}

/// Right pane in the footer: the running timer display.
pub struct RunningTimer {
    wrapper: GtkBox,
    timer_box: GtkBox,
    hms_label: Label,
    ms_label: Label,
}

impl RunningTimer {
    pub fn new(timer: &Timer, config: &mut Config) -> Self {
        let wrapper = GtkBox::builder()
            .orientation(Horizontal)
            .halign(Align::End)
            .build();

        let timer_box = GtkBox::new(Horizontal, 0);
        timer_box.add_css_class("timer");
        if timer.current_phase() == TimerPhase::Running {
            timer_box.add_css_class("active-timer");
        } else {
            timer_box.add_css_class("inactive-timer");
        }

        let formatted = config.format.timer.format_timer(timer);
        let (left, right) = if let Some((l, r)) = formatted.rsplit_once('.') {
            (format!("{l}."), r.to_owned())
        } else {
            (formatted.clone(), String::new())
        };

        let hms_label = Label::builder().label(left).build();
        hms_label.add_css_class("bigtimer");

        let ms_label = Label::builder().label(right).margin_top(14).build();
        ms_label.add_css_class("smalltimer");

        timer_box.append(&hms_label);
        timer_box.append(&ms_label);
        wrapper.append(&timer_box);

        Self {
            wrapper,
            timer_box,
            hms_label,
            ms_label,
        }
    }

    pub fn container(&self) -> &GtkBox {
        &self.wrapper
    }

    pub fn update(&mut self, timer: &Timer, config: &mut Config) {
        self.rebuild(timer, config);
    }

    fn rebuild(&mut self, timer: &Timer, config: &mut Config) {
        self.timer_box.set_css_classes(match timer.current_phase() {
            TimerPhase::Running => &["timer", "active-timer"],
            _ => &["timer", "inactive-timer"],
        });

        // Update labels only if changed
        let formatted = config.format.timer.format_timer(timer);
        let (left, right) = if let Some((l, r)) = formatted.rsplit_once('.') {
            (format!("{l}."), r.to_owned())
        } else {
            (formatted.clone(), String::new())
        };

        if self.hms_label.label().as_str() != left {
            self.hms_label.set_label(&left);
        }
        if self.ms_label.label().as_str() != right {
            self.ms_label.set_label(&right);
        }
    }
}
