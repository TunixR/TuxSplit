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
use livesplit_core::{Run, TimeSpan};
use std::sync::{Arc, RwLock};

use adw::prelude::*;
use adw::{
    ComboRow, EntryRow, HeaderBar, PreferencesGroup, PreferencesPage, ToolbarView, ViewStack,
    ViewSwitcher, Window,
};

#[derive(Clone)]
pub struct SplitEditor {
    dialog: ToolbarView,
    run_snapshot: Arc<RwLock<Run>>,
}

impl SplitEditor {
    pub fn new() -> Self {
        let ctx = TuxSplitContext::get_instance();

        let dialog = ToolbarView::new();

        let run_snapshot = {
            let run = ctx.get_run();
            Arc::new(RwLock::new(run))
        };

        let this = Self {
            dialog,
            run_snapshot,
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
        ctx.connect_local("run-changed", false, move |_| {
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
        let snapshot_binding = self.run_snapshot.clone();
        let action_bar_binding = action_bar.clone();
        save_button.connect_clicked(move |_| {
            if let Ok(mut snapshot) = snapshot_binding.try_write() {
                *snapshot = TuxSplitContext::get_instance().get_run();
            }
            action_bar_binding.set_revealed(false);
        });

        // Connect cancel button
        let snapshot_binding = Arc::clone(&self.run_snapshot);
        let action_bar_binding = action_bar.clone();
        cancel_button.connect_clicked(move |_| {
            TuxSplitContext::get_instance().set_run(snapshot_binding.read().unwrap().clone());
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

        let name = EntryRow::builder()
            .title("Game Name")
            .text(self.run_snapshot.read().unwrap().game_name())
            .build();
        let category = EntryRow::builder()
            .title("Category")
            .text(self.run_snapshot.read().unwrap().category_name())
            .build();

        {
            name.connect_text_notify(move |entry| {
                let new_name = entry.text().to_string();
                let ctx = TuxSplitContext::get_instance();

                let mut run = ctx.get_run();

                run.set_game_name(new_name);

                ctx.set_run(run);
            });
        }
        {
            category.connect_text_notify(move |entry| {
                let new_category = entry.text().to_string();
                let ctx = TuxSplitContext::get_instance();

                let mut run = ctx.get_run();

                run.set_category_name(new_category);

                ctx.set_run(run);
            });
        }

        group.add(&name);
        group.add(&category);

        group
    }

    fn build_timer_preferences(&self) -> PreferencesGroup {
        let ctx = TuxSplitContext::get_instance();
        let timer = {
            let shared = ctx.timer();
            shared.read().unwrap().clone()
        };
        let current_method_index = match timer.current_timing_method() {
            livesplit_core::TimingMethod::GameTime => 1,
            _ => 0,
        };

        let group = PreferencesGroup::builder()
            .title("Timer")
            .description("Run timing configuration")
            .build();

        let options = StringList::new(&["Real Time", "Game Time"]);
        let initial_method = current_method_index;

        let offset_str = format!("{:3}", timer.run().offset().total_seconds(),);
        let offset = EntryRow::builder()
            .title("Start at")
            .text(offset_str)
            .build();
        let timing_method = ComboRow::builder()
            .title("Timing Method")
            .model(&options)
            .selected(initial_method)
            .build();

        offset.connect_text_notify(move |entry| {
            // Offset must be a valid f64 value
            if entry.text().parse::<f64>().is_ok() {
                entry.set_title("Start at");
                entry.remove_css_class("error");
                let new_offset = entry.text().parse::<f64>().unwrap();

                let ctx = TuxSplitContext::get_instance();
                let mut run = ctx.get_run();

                run.set_offset(TimeSpan::from_seconds(new_offset));

                ctx.set_run(run);
            } else {
                entry.set_title("Start at (entry must be a valid number)");
                entry.add_css_class("error");
            }
        });

        timing_method.connect_selected_notify(move |r| {
            let ctx = TuxSplitContext::get_instance();

            let idx = r.selected();
            if let Ok(mut t) = ctx.timer().try_write() {
                match idx {
                    0 => t.set_current_timing_method(livesplit_core::TimingMethod::RealTime),
                    1 => t.set_current_timing_method(livesplit_core::TimingMethod::GameTime),
                    _ => (),
                }
                drop(t);
                ctx.emit_by_name::<()>("run-changed", &[]);
            }
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

        let editor_ctx = EditorContext::new();
        let segment_editor = SegmentsEditor::new(editor_ctx);
        group.add(segment_editor.container());

        page.add(&group);

        page
    }
}
