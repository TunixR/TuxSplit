use crate::config::Config;
use livesplit_core::{Timer, analysis::sum_of_segments::best::calculate as calculate_sob};

use tracing::debug;

pub fn current_attempt_running_duration(timer: &Timer) -> time::Duration {
    use livesplit_core::TimingMethod;
    let current_dur = timer
        .current_attempt_duration()
        .to_duration()
        .checked_add(timer.run().offset().to_duration())
        .unwrap_or_default();

    let paused_time = timer.get_pause_time().unwrap_or_default().to_duration();

    let loading_times = if timer.current_timing_method() == TimingMethod::GameTime {
        timer.loading_times().to_duration()
    } else {
        time::Duration::ZERO
    };

    current_dur
        .checked_sub(paused_time)
        .unwrap_or_default()
        .checked_sub(loading_times)
        .unwrap_or_default()
}

pub fn real_time_sob(timer: &Timer) -> time::Duration {
    let mut predictions = vec![None; timer.run().len() + 1];
    let predictions = &mut predictions[..];
    calculate_sob(
        timer.run().segments(),
        predictions,
        false,
        true,
        timer.current_timing_method(),
    )
    .unwrap_or_default()
    .to_duration()
}

pub fn best_segment_duration(segment: &livesplit_core::Segment, timer: &Timer) -> time::Duration {
    use livesplit_core::TimingMethod;
    if timer.current_timing_method() == TimingMethod::GameTime {
        segment
            .best_segment_time()
            .game_time
            .unwrap_or_default()
            .to_duration()
    } else {
        segment
            .best_segment_time()
            .real_time
            .unwrap_or_default()
            .to_duration()
    }
}

pub fn segment_split_time(segment: &livesplit_core::Segment, timer: &Timer) -> time::Duration {
    use livesplit_core::TimingMethod;
    if timer.current_timing_method() == TimingMethod::GameTime {
        segment
            .split_time()
            .game_time
            .unwrap_or_default()
            .to_duration()
    } else {
        segment
            .split_time()
            .real_time
            .unwrap_or_default()
            .to_duration()
    }
}

pub fn segment_best_time(segment: &livesplit_core::Segment, timer: &Timer) -> time::Duration {
    segment
        .comparison_timing_method("Best Segments", timer.current_timing_method())
        .unwrap_or_default()
        .to_duration()
}

pub fn segment_comparison_time(segment: &livesplit_core::Segment, timer: &Timer) -> time::Duration {
    segment
        .comparison_timing_method(timer.current_comparison(), timer.current_timing_method())
        .unwrap_or_default()
        .to_duration()
}

pub fn previous_comparison_values(timer: &Timer, index: usize) -> (time::Duration, time::Duration) {
    use livesplit_core::TimingMethod;
    let segments = timer.run().segments();
    if index > 0 {
        let prev = &segments[index - 1];
        let prev_comp_duration = prev
            .comparison_timing_method(timer.current_comparison(), timer.current_timing_method())
            .unwrap_or_default()
            .to_duration();
        let prev_split_time = if timer.current_timing_method() == TimingMethod::GameTime {
            prev.split_time()
                .game_time
                .unwrap_or_default()
                .to_duration()
        } else {
            prev.split_time()
                .real_time
                .unwrap_or_default()
                .to_duration()
        };
        (prev_comp_duration, prev_split_time)
    } else {
        (time::Duration::ZERO, time::Duration::ZERO)
    }
}

pub fn best_comparison_values(timer: &Timer, index: usize) -> (time::Duration, time::Duration) {
    use livesplit_core::TimingMethod;
    let segments = timer.run().segments();
    if index > 0 {
        let prev = &segments[index - 1];
        let prev_best_duration = prev
            .comparison_timing_method("Best Segments", timer.current_timing_method())
            .unwrap_or_default()
            .to_duration();
        let prev_split_time = if timer.current_timing_method() == TimingMethod::GameTime {
            prev.split_time()
                .game_time
                .unwrap_or_default()
                .to_duration()
        } else {
            prev.split_time()
                .real_time
                .unwrap_or_default()
                .to_duration()
        };
        (prev_best_duration, prev_split_time)
    } else {
        (time::Duration::ZERO, time::Duration::ZERO)
    }
}

pub fn format_signed(diff: time::Duration, config: &mut Config) -> String {
    let sign = if diff.is_positive() {
        "+"
    } else if diff.is_negative() {
        "-"
    } else {
        "~"
    };
    let abs = diff.abs();
    let formatted = config.format.split.format_segment_time(&abs);
    format!("{sign}{formatted}")
}

pub fn classify_split_label(
    comparison_duration: time::Duration,
    split_duration: time::Duration,
    diff: time::Duration,
    goldsplit_duration: time::Duration,
    running: bool,
) -> &'static str {
    if (split_duration < goldsplit_duration || goldsplit_duration == time::Duration::ZERO)
        && !running
    {
        "goldsplit"
    } else if diff.is_negative() {
        if split_duration <= comparison_duration {
            "greensplit"
        } else {
            "lostgreensplit"
        }
    } else if diff.is_positive() {
        if split_duration <= comparison_duration {
            "gainedredsplit"
        } else {
            "redsplit"
        }
    } else {
        "" // how
    }
}

#[cfg(test)]
mod classify_split_labels_tests {
    use super::*;
    use time::Duration;

    #[test]
    fn classify_gold_when_first_split() {
        let comparison = Duration::ZERO;
        let split_duration = Duration::seconds(8);
        let diff = Duration::ZERO;
        let gold = Duration::ZERO;

        let class = classify_split_label(comparison, split_duration, diff, gold, false);
        assert!(class == "goldsplit", "Expected goldsplit: got {class:?}",);
    }

    #[test]
    fn classify_gold_when_not_running_and_new_best_and_ahead() {
        let comparison = Duration::seconds(10);
        let split_duration = Duration::seconds(8);
        let diff = Duration::seconds(-2);
        let gold = Duration::seconds(9);

        let class = classify_split_label(comparison, split_duration, diff, gold, false);
        assert!(class == "goldsplit", "Expected goldsplit: got {class:?}",);
    }

    #[test]
    fn classify_gold_when_not_running_and_zero_gold_duration() {
        // When gold duration is ZERO and not running, we treat it as gold (first split behavior)
        let comparison = Duration::ZERO;
        let split_duration = Duration::seconds(12);
        let diff = Duration::ZERO;
        let gold = Duration::ZERO;

        let class = classify_split_label(comparison, split_duration, diff, gold, false);
        assert!(
            class == "goldsplit",
            "Expected goldsplit when gold duration is zero and not running: got {class:?}",
        );
    }

    #[test]
    fn classify_gainedred_when_not_running_and_behind_and_ahead_comparison() {
        let comparison = Duration::seconds(10);
        let split_duration = Duration::seconds(9);
        let diff = Duration::seconds(1);
        let gold = Duration::seconds(8);

        let class = classify_split_label(comparison, split_duration, diff, gold, false);
        assert!(
            class == "gainedredsplit",
            "Expected redsplit when behind and gaining: got {class:?}",
        );
    }

    #[test]
    fn classify_red_when_not_running_and_behind_and_behind_comparison() {
        let comparison = Duration::seconds(10);
        let split_duration = Duration::seconds(11);
        let diff = Duration::seconds(1);
        let gold = Duration::seconds(9);

        let class = classify_split_label(comparison, split_duration, diff, gold, false);
        assert!(
            class == "redsplit",
            "Expected redsplit when behind and not gaining: got {class:?}",
        );
    }

    #[test]
    fn classify_green_when_ahead_and_split_on_or_under_comparison_duration() {
        let comparison = Duration::seconds(10);
        let split_duration = Duration::seconds(9);
        let diff = Duration::seconds(-1);
        let gold = Duration::seconds(8);

        let class = classify_split_label(comparison, split_duration, diff, gold, false);
        assert!(
            class == "greensplit",
            "Expected greensplit when ahead and not losing against comparison_duration: got {class:?}",
        );
    }

    #[test]
    fn classify_lost_green_when_ahead_but_split_exceeds_comparison_duration() {
        let comparison = Duration::seconds(10);
        let split_duration = Duration::seconds(11); // longer than comparison_duration
        let diff = Duration::seconds(-1); // still ahead overall vs segment comparison target
        let gold = Duration::seconds(8);

        let class = classify_split_label(comparison, split_duration, diff, gold, false);
        assert!(
            class == "lostgreensplit",
            "Expected lostgreensplit when ahead (negative diff) but split exceeds comparison_duration: got {class:?}",
        );
    }

    #[test]
    fn classify_no_color_when_diff_is_zero() {
        let comparison = Duration::seconds(10);
        let split_duration = Duration::seconds(10);
        let diff = Duration::ZERO;
        let gold = Duration::seconds(5);

        let class = classify_split_label(comparison, split_duration, diff, gold, false);
        assert!(
            class.is_empty(),
            "Expected no red/green class when diff is zero: got {class:?}",
        );
    }
}
