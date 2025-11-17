mod action_bar;
mod context;
mod model;
mod row;
mod table;
pub use context::EditorContext;
pub use model::SegmentsModel;

use crate::context::TuxSplitContext;
use crate::ui::editor::table::SegmentsEditor;
use gtk4::{ActionBar, StringList};
use livesplit_core::{Run, TimeSpan, Timer};
use std::sync::{Arc, RwLock};

use adw::prelude::*;
use adw::{
    ComboRow, EntryRow, HeaderBar, PreferencesGroup, PreferencesPage, ToolbarView, ViewStack,
    ViewSwitcher, Window,
};

#[derive(Clone)]
pub struct SplitEditor {
    dialog: ToolbarView,
    timer: Arc<RwLock<Timer>>,
    run_snapshot: Arc<RwLock<Run>>,
    ctx: Arc<TuxSplitContext>,
}

impl SplitEditor {
    pub fn new(timer: Arc<RwLock<Timer>>, ctx: Arc<TuxSplitContext>) -> Self {
        let dialog = ToolbarView::new();

        let run_snapshot = {
            let timer_read = timer.read().unwrap();
            let snapshot = timer_read.run().clone();
            Arc::new(RwLock::new(snapshot))
        };

        let this = Self {
            dialog,
            timer,
            run_snapshot,
            ctx,
        };

        let run_info = this.build_run_info_page();
        let segment_editor = this.build_segment_editor_page();

        let content = ViewStack::builder().build();
        content
            .add_titled(&run_info, None, "Run")
            .set_icon_name(Some("gears-symbolic"));
        content
            .add_titled(&segment_editor, None, "Segments")
            .set_icon_name(Some("view-list-symbolic"));

        let headerbar = HeaderBar::builder().show_end_title_buttons(true).build();
        let switcher = ViewSwitcher::builder()
            .stack(&content)
            .policy(adw::ViewSwitcherPolicy::Wide)
            .build();
        headerbar.set_title_widget(Some(&switcher));

        let action_bar = this.build_cancel_banner();

        this.dialog.add_top_bar(&headerbar);
        this.dialog.set_content(Some(&content));
        this.dialog.add_bottom_bar(&action_bar);
        this.dialog.set_bottom_bar_style(adw::ToolbarStyle::Raised);
        this.dialog.set_extend_content_to_bottom_edge(true); // Content below action bar

        // Call show Cancel/save on run-changed
        let action_bar_binding = action_bar.clone();
        this.ctx.connect_local("run-changed", false, move |_| {
            if !action_bar_binding.is_revealed() {
                // cancel will trigger run-changed
                action_bar_binding.set_revealed(true);
            }
            None
        });

        this
    }

    pub fn dialog(&self) -> &ToolbarView {
        &self.dialog
    }

    pub fn present(&self) {
        let window = Window::builder()
            .height_request(700) // Arbitrary I know
            .width_request(800) // Arbitrary I know
            .build();
        window.set_content(Some(self.dialog()));
        window.present();
    }

    fn build_cancel_banner(&self) -> ActionBar {
        let action_bar = ActionBar::builder()
            .css_classes(["undershoot-top", "undershoot-bottom"])
            .margin_bottom(6)
            .margin_top(6)
            .margin_start(6)
            .margin_end(6)
            .revealed(false)
            .build();

        let save_button = gtk4::Button::builder()
            .css_classes(["suggested-action"])
            .label("Save")
            .focus_on_click(true)
            .build();
        let cancel_button = gtk4::Button::builder()
            .label("Cancel")
            .focus_on_click(true)
            .build();

        // Connect save button
        let timer_binding = Arc::clone(&self.timer);
        let snapshot_binding = self.run_snapshot.clone();
        let action_bar_binding = action_bar.clone();
        save_button.connect_clicked(move |_| {
            let t = timer_binding.read().unwrap();
            if let Ok(mut snapshot) = snapshot_binding.try_write() {
                *snapshot = t.run().clone();
            }
            action_bar_binding.set_revealed(false);
        });

        // Connect cancel button
        let snapshot_binding = Arc::clone(&self.run_snapshot);
        let timer_binding = Arc::clone(&self.timer);
        let action_bar_binding = action_bar.clone();
        let context_binding = self.ctx.clone();
        cancel_button.connect_clicked(move |_| {
            let t = timer_binding.try_write();
            if let Ok(mut timer) = t {
                let snapshot = snapshot_binding.read().unwrap();
                assert!(timer.set_run(snapshot.clone()).is_ok());
            }
            context_binding.emit_by_name::<()>("run-changed", &[]);
            action_bar_binding.set_revealed(false);
        });

        action_bar.pack_start(&cancel_button);
        action_bar.pack_end(&save_button);

        action_bar
    }

    fn build_run_info_page(&self) -> PreferencesPage {
        let page = PreferencesPage::builder().title("General").build();

        let run_info_group = self.build_run_info_preferences();
        let timer_group = self.build_timer_preferences();
        // let autosplit_group = self.build_autosplit_preferences();

        page.add(&run_info_group);
        page.add(&timer_group);
        // page.add(&autosplit_group);

        page
    }

    fn build_run_info_preferences(&self) -> PreferencesGroup {
        // Logic to create preferences for run information
        let group = PreferencesGroup::builder()
            .title("Run Information")
            .description("General run information details")
            .build();

        let timer = self.timer.read().unwrap();
        let name = EntryRow::builder()
            .title("Game Name")
            .text(timer.run().game_name())
            .build();
        let category = EntryRow::builder()
            .title("Category")
            .text(timer.run().category_name())
            .build();

        {
            let timer_binding = Arc::clone(&self.timer);
            let context_binding = self.ctx.clone();
            name.connect_text_notify(move |entry| {
                let new_name = entry.text().to_string();

                let mut timer = timer_binding.write().unwrap();
                let mut run = timer.run().clone();

                run.set_game_name(new_name);
                assert!(timer.set_run(run).is_ok());

                drop(timer);
                context_binding.emit_by_name::<()>("run-changed", &[]);
            });
        }
        {
            let timer_binding = Arc::clone(&self.timer);
            let context_binding = self.ctx.clone();
            category.connect_text_notify(move |entry| {
                let new_category = entry.text().to_string();

                let mut timer = timer_binding.write().unwrap();
                let mut run = timer.run().clone();

                run.set_category_name(new_category);
                assert!(timer.set_run(run).is_ok());

                drop(timer);
                context_binding.emit_by_name::<()>("run-changed", &[]);
            });
        }

        group.add(&name);
        group.add(&category);

        group
    }

    fn build_timer_preferences(&self) -> PreferencesGroup {
        let group = PreferencesGroup::builder()
            .title("Timer")
            .description("Run timing configuration")
            .build();

        let options = StringList::new(&["Real Time", "Game Time"]);
        let initial_method = {
            let timer = self.timer.read().unwrap();
            match timer.current_timing_method() {
                livesplit_core::TimingMethod::GameTime => 1,
                _ => 0,
            }
        };

        let timer = self.timer.read().unwrap();

        let offset = EntryRow::builder()
            .title("Start at")
            .text(format!("{:3}", timer.run().offset().total_seconds(),))
            .build();
        let timing_method = ComboRow::builder()
            .title("Timing Method")
            .model(&options)
            .selected(initial_method)
            .build();

        let timer_binding = Arc::clone(&self.timer);
        let context_binding = self.ctx.clone();
        offset.connect_text_notify(move |entry| {
            // Offset must be a valid f64 value
            if entry.text().parse::<f64>().is_ok() {
                entry.set_title("Start at");
                entry.remove_css_class("error");
                let new_offset = entry.text().parse::<f64>().unwrap();

                let mut timer = timer_binding.write().unwrap();
                let mut run = timer.run().clone();

                run.set_offset(TimeSpan::from_seconds(new_offset));
                assert!(timer.set_run(run).is_ok());

                drop(timer);
                context_binding.emit_by_name::<()>("run-changed", &[]);
            } else {
                entry.set_title("Start at (entry must be a valid number)");
                entry.add_css_class("error");
            }
        });

        let timer_binding = Arc::clone(&self.timer);
        let context_binding = self.ctx.clone();
        timing_method.connect_selected_notify(move |r| {
            let idx = r.selected();
            let mut t = timer_binding.write().unwrap();
            match idx {
                0 => t.set_current_timing_method(livesplit_core::TimingMethod::RealTime),
                1 => t.set_current_timing_method(livesplit_core::TimingMethod::GameTime),
                _ => (),
            }

            drop(t);
            context_binding.emit_by_name::<()>("run-changed", &[]);
        });

        group.add(&offset);
        group.add(&timing_method);

        group
    }

    fn build_autosplit_preferences(&self) -> PreferencesGroup {
        // Logic to create autosplitter preferences UI component
        unimplemented!()
    }

    fn build_segment_editor_page(&self) -> PreferencesPage {
        let page = PreferencesPage::builder().title("Segments").build();

        let group = PreferencesGroup::builder()
            .title("Segment Editor")
            .description("Edit your run segments")
            .build();

        let editor_ctx = EditorContext::new(self.timer.clone(), Some(self.ctx.clone()));
        let segment_editor = SegmentsEditor::new(editor_ctx);
        group.add(segment_editor.container());

        page.add(&group);

        page
    }
}
