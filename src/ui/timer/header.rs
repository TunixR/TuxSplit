use crate::config::Config;

use adw::prelude::*;
use gtk4::{Align, Box as GtkBox, Label, Orientation::Vertical};

use livesplit_core::Timer;

/// TimerHeader
/// Renders the top section of the timer UI:
/// - Game name (styled as `title-2`)
/// - Category (styled as `heading`)
///
/// This component owns a stable container widget that can be appended to the main layout.
pub struct TimerHeader {
    container: GtkBox,
    run_info: RunInfo,
}

impl TimerHeader {
    /// Create a new header component initialized from the given timer.
    pub fn new(timer: &Timer) -> Self {
        // Root container (header-level)
        let container = GtkBox::builder()
            .orientation(Vertical)
            .halign(Align::Center)
            .build();

        // Run info (game + category)
        let run_info = RunInfo::new(timer);

        container.append(run_info.container());

        Self {
            container,
            run_info,
        }
    }

    /// Access the GTK container to attach this component in the parent UI.
    pub fn container(&self) -> &GtkBox {
        &self.container
    }

    /// Update the header from the current timer/config state.
    /// Currently only the timer is used (to update game/category labels).
    pub fn refresh(&mut self, timer: &Timer, _config: &mut Config) {
        self.run_info.update(timer);
    }
}

/// RunInfo
///
/// Holds and renders:
/// - Game name (Label with CSS class `title-2`)
/// - Category (Label with CSS class `heading`)
pub struct RunInfo {
    container: GtkBox,
    run_name: Label,
    category: Label,
}

impl RunInfo {
    /// Build the run info UI from the timer.
    pub fn new(timer: &Timer) -> Self {
        let container = GtkBox::builder()
            .orientation(Vertical)
            .halign(Align::Center)
            .build();

        let run_name = Label::builder().label(timer.run().game_name()).build();
        run_name.add_css_class("title-2");

        let category = Label::builder().label(timer.run().category_name()).build();
        category.add_css_class("heading");

        container.append(&run_name);
        container.append(&category);

        Self {
            container,
            run_name,
            category,
        }
    }

    /// Access the GTK container to attach this component in the parent header.
    pub fn container(&self) -> &GtkBox {
        &self.container
    }

    /// Update labels using the current timer state.
    pub fn update(&self, timer: &Timer) {
        self.run_name.set_label(timer.run().game_name());
        self.category.set_label(timer.run().category_name());
    }
}
