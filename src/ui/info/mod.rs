use crate::config::Config;
use crate::utils::comparisons::{
    best_comparison_values, best_segment_duration, classify_split_label,
    current_attempt_running_duration, format_signed, previous_comparison_values,
    previous_split_combined_gold_and_prev_comparison, real_time_sob, segment_best_time,
    segment_comparison_time, segment_split_time,
};

use gtk4::{CenterBox, Label, Orientation::Horizontal, prelude::WidgetExt};

use livesplit_core::Timer;
use livesplit_core::analysis::{current_pace, pb_chance, total_playtime};

pub enum AdditionalInfoKind {
    PrevSegmentDiff,
    PrevSegmentBest,
    BestPossibleTime,
    PossibleTimeSave,
    CurrentPace,
    TotalPlaytime,
    PbChance,
}

pub static ALL_ADDITIONAL_INFOS: [AdditionalInfoKind; 7] = [
    AdditionalInfoKind::PrevSegmentDiff,
    AdditionalInfoKind::PrevSegmentBest,
    AdditionalInfoKind::BestPossibleTime,
    AdditionalInfoKind::PossibleTimeSave,
    AdditionalInfoKind::CurrentPace,
    AdditionalInfoKind::TotalPlaytime,
    AdditionalInfoKind::PbChance,
];

pub trait AdditionalInfo {
    fn new(timer: &Timer, config: &Config) -> Self
    where
        Self: Sized;
    fn update(&mut self, timer: &Timer, config: &Config);
    fn container(&self) -> &CenterBox;
}

pub struct PrevSegmentDiffInfo {
    container: CenterBox,
    value: Label,
}

pub struct PrevSegmentBestInfo {
    container: CenterBox,
    value: Label,
}

pub struct BestPossibleTimeInfo {
    container: CenterBox,
    value: Label,
}

pub struct PossibleTimeSaveInfo {
    container: CenterBox,
    value: Label,
}

pub struct CurrentPaceInfo {
    container: CenterBox,
    value: Label,
}

pub struct PbChanceInfo {
    container: CenterBox,
    value: Label,
}

pub struct TotalPlaytimeInfo {
    container: CenterBox,
    value: Label,
}

impl AdditionalInfo for PrevSegmentDiffInfo {
    fn new(timer: &Timer, config: &Config) -> Self {
        let container = CenterBox::builder().orientation(Horizontal).build();

        let label = Label::builder()
            .label("Previous Segment:")
            .css_classes(["heading"])
            .build();
        let value = Label::builder().label("").css_classes(["timer"]).build();

        container.set_start_widget(Some(&label));
        container.set_end_widget(Some(&value));

        let mut res = Self { container, value };

        res.update(timer, config); // Initialize with default timer state

        res
    }

    fn update(&mut self, timer: &Timer, config: &Config) {
        self.value.set_css_classes(&[]);
        self.value.set_label("");
        if let Some(mut index) = timer.current_split_index()
            && index > 0
        {
            index -= 1; // Previous segment index

            let segment = &timer.run().segments()[index];

            let segment_comparison_time = segment_comparison_time(segment, timer);
            let (previous_comparison_duration, previous_split_time) =
                previous_comparison_values(timer, index);
            let segment_comparison_duration = segment_comparison_time
                .checked_sub(previous_comparison_duration)
                .unwrap_or_default()
                .abs();

            let split_time = segment_split_time(segment, timer);

            if split_time == time::Duration::ZERO {
                self.value.set_label("");
            } else {
                let diff = split_time
                    .checked_sub(previous_split_time)
                    .unwrap_or_default()
                    .checked_sub(segment_comparison_duration)
                    .unwrap_or_default();

                if segment_comparison_time != time::Duration::ZERO {
                    self.value.set_label(format_signed(diff, config).as_str());

                    let gold_duration = best_segment_duration(segment, timer);
                    let split_duration = split_time
                        .checked_sub(previous_split_time)
                        .unwrap_or_default();

                    self.value.add_css_class(classify_split_label(
                        segment_comparison_duration,
                        split_duration,
                        diff,
                        gold_duration,
                        false,
                    ));
                }
            }
        } else {
            self.value.set_label("");
        }
    }

    fn container(&self) -> &CenterBox {
        &self.container
    }
}

impl AdditionalInfo for PrevSegmentBestInfo {
    fn new(timer: &Timer, config: &Config) -> Self {
        let container = CenterBox::builder().orientation(Horizontal).build();

        let label = Label::builder()
            .label("Previous Segment (Best):")
            .css_classes(["heading"])
            .build();
        let value = Label::builder().label("").css_classes(["timer"]).build();

        container.set_start_widget(Some(&label));
        container.set_end_widget(Some(&value));

        let mut res = Self { container, value };

        res.update(timer, config); // Initialize with default timer state

        res
    }

    fn update(&mut self, timer: &Timer, config: &Config) {
        self.value.set_css_classes(&[]);
        self.value.set_label("");
        if let Some(mut index) = timer.current_split_index()
            && index > 0
        {
            index -= 1; // Previous segment index

            let segment = &timer.run().segments()[index];

            let segment_best_time = segment_best_time(segment, timer);
            let (_, previous_split_time) = previous_comparison_values(timer, index);
            let (previous_best_duration, previous_best_time) = best_comparison_values(timer, index);
            let segment_best_duration = segment_best_time
                .checked_sub(previous_best_duration)
                .unwrap_or_default()
                .abs();

            let split_time = segment_split_time(segment, timer);

            if split_time == time::Duration::ZERO {
                self.value.set_label("");
            } else {
                let diff = split_time
                    .checked_sub(previous_split_time)
                    .unwrap_or_default()
                    .checked_sub(segment_best_duration)
                    .unwrap_or_default();

                if segment_best_time != time::Duration::ZERO {
                    self.value.set_label(format_signed(diff, config).as_str());

                    let gold_duration = best_segment_duration(segment, timer);
                    let split_duration = split_time
                        .checked_sub(previous_best_time)
                        .unwrap_or_default();

                    self.value.add_css_class(classify_split_label(
                        segment_best_duration,
                        split_duration,
                        diff,
                        gold_duration,
                        false,
                    ));
                }
            }
        } else {
            self.value.set_label("");
        }
    }

    fn container(&self) -> &CenterBox {
        &self.container
    }
}

impl AdditionalInfo for BestPossibleTimeInfo {
    fn new(timer: &Timer, config: &Config) -> Self {
        let container = CenterBox::builder().orientation(Horizontal).build();

        let label = Label::builder()
            .label("Best Possible Time:")
            .css_classes(["heading"])
            .build();
        let value = Label::builder().label("").css_classes(["timer"]).build();

        container.set_start_widget(Some(&label));
        container.set_end_widget(Some(&value));

        let mut res = Self { container, value };

        res.update(timer, config); // Initialize with default timer state

        res
    }

    fn update(&mut self, timer: &Timer, config: &Config) {
        if timer.current_phase().is_not_running() {
            self.value.set_label("");
        } else if timer.current_phase().is_running() || timer.current_phase().is_paused() {
            let segment = timer.current_split().unwrap_or(timer.run().segment(0));

            let segment_best_duration = segment_best_time(segment, timer);

            // Diff to SOB
            let diff = current_attempt_running_duration(timer)
                .checked_sub(segment_best_duration)
                .unwrap_or_default();

            // We will be adding only diff time to the best possible time when we are behind
            let live_addition = if diff.is_positive() {
                diff
            } else {
                time::Duration::ZERO
            };

            let best_possible_time = real_time_sob(timer)
                .checked_add(live_addition)
                .unwrap_or_default();
            if best_possible_time == time::Duration::ZERO {
                self.value.set_label("");
            } else {
                self.value.set_label(
                    config
                        .format
                        .segment
                        .format_duration(&best_possible_time)
                        .as_str(),
                );
            }
        } else if timer.current_phase().is_ended() {
            self.value.set_label(
                config
                    .format
                    .segment
                    .format_duration(&current_attempt_running_duration(timer))
                    .as_str(),
            );
        }
    }

    fn container(&self) -> &CenterBox {
        &self.container
    }
}

impl AdditionalInfo for PossibleTimeSaveInfo {
    fn new(timer: &Timer, config: &Config) -> Self {
        let container = CenterBox::builder().orientation(Horizontal).build();

        let label = Label::builder()
            .label("Possible Time Save:")
            .css_classes(["heading"])
            .build();
        let value = Label::builder().label("").css_classes(["timer"]).build();

        container.set_start_widget(Some(&label));
        container.set_end_widget(Some(&value));

        let mut res = Self { container, value };

        res.update(timer, config); // Initialize with default timer state

        res
    }

    fn update(&mut self, timer: &Timer, config: &Config) {
        if timer.current_phase().is_not_running() {
            self.value.set_label("");
        } else if timer.current_phase().is_running() || timer.current_phase().is_paused() {
            let index = timer.current_split_index().unwrap_or(0);

            let (_, combined_gold, previous_comparion_time) =
                previous_split_combined_gold_and_prev_comparison(timer, index);
            let current_comparison_time = segment_comparison_time(
                timer.current_split().unwrap_or(timer.run().segment(0)),
                timer,
            );

            // Diff from gold to comp. This is the possible time save
            let gold_diff = current_comparison_time
                .checked_sub(previous_comparion_time)
                .unwrap_or_default()
                .checked_sub(combined_gold)
                .unwrap_or_default();

            self.value.set_label(
                config
                    .format
                    .comparison
                    .format_duration(&gold_diff)
                    .as_str(),
            );
        } else if timer.current_phase().is_ended() {
            self.value.set_label("");
        }
    }

    fn container(&self) -> &CenterBox {
        &self.container
    }
}

impl AdditionalInfo for CurrentPaceInfo {
    fn new(timer: &Timer, config: &Config) -> Self {
        let container = CenterBox::builder().orientation(Horizontal).build();

        let label = Label::builder()
            .label("Current Pace:")
            .css_classes(["heading"])
            .build();
        let value = Label::builder().label("").css_classes(["timer"]).build();

        container.set_start_widget(Some(&label));
        container.set_end_widget(Some(&value));

        let mut res = Self { container, value };

        res.update(timer, config); // Initialize with default timer state

        res
    }

    fn update(&mut self, timer: &Timer, config: &Config) {
        if timer.current_phase().is_not_running() {
            self.value.set_label("");
        } else {
            let timer_snaptshot = timer.snapshot();
            let pace = current_pace::calculate(&timer_snaptshot, timer.current_comparison())
                .0
                .unwrap_or_default();
            let pace = config.format.timer.format_time_span(&pace);
            self.value.set_label(&pace);
        }
    }

    fn container(&self) -> &CenterBox {
        &self.container
    }
}

impl AdditionalInfo for PbChanceInfo {
    fn new(timer: &Timer, config: &Config) -> Self {
        let container = CenterBox::builder().orientation(Horizontal).build();

        let label = Label::builder()
            .label("PB Chance:")
            .css_classes(["heading"])
            .build();
        let value = Label::builder().label("").css_classes(["timer"]).build();

        container.set_start_widget(Some(&label));
        container.set_end_widget(Some(&value));

        let mut res = Self { container, value };

        res.update(timer, config); // Initialize with default timer state

        res
    }

    fn update(&mut self, timer: &Timer, _config: &Config) {
        if timer.current_phase().is_not_running() {
            self.value.set_label("");
        } else {
            let chance = pb_chance::for_timer(&timer.snapshot()).0;
            self.value
                .set_label(format!("{:.2}%", chance * 100.0).as_str());
        }
    }

    fn container(&self) -> &CenterBox {
        &self.container
    }
}

impl AdditionalInfo for TotalPlaytimeInfo {
    fn new(timer: &Timer, config: &Config) -> Self {
        let container = CenterBox::builder().orientation(Horizontal).build();

        let label = Label::builder()
            .label("Total Playtime:")
            .css_classes(["heading"])
            .build();
        let value = Label::builder().label("").css_classes(["timer"]).build();

        container.set_start_widget(Some(&label));
        container.set_end_widget(Some(&value));

        let mut res = Self { container, value };

        res.update(timer, config); // Initialize with default timer state

        res
    }

    fn update(&mut self, timer: &Timer, config: &Config) {
        let playtime = total_playtime::calculate(timer);
        self.value
            .set_label(&config.format.comparison.format_time_span(&playtime));
    }

    fn container(&self) -> &CenterBox {
        &self.container
    }
}
