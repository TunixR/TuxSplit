//! Global application context providing shared access to the Timer, Config,
//! Runtime (auto-splitting), and a signal bus for run mutations.

use std::cell::RefCell;
use std::sync::{Arc, RwLock};

use glib::prelude::*;
use glib::{subclass::Signal, subclass::prelude::*};
use std::sync::OnceLock;

use std::env;
use std::path::{Path, PathBuf};

use gtk4::gio;

use adw::prelude::*;
use adw::{Application, ApplicationWindow, ToolbarView};

use tracing::debug;
use tracing::info;

use livesplit_core::{Run, SharedTimer, Timer, auto_splitting::Runtime};

use crate::config::Config;
use crate::ui::TuxSplitHeader;
use crate::ui::timer::TuxSplitTimer;

mod imp {
    use super::*;

    pub struct TuxSplitContext {
        pub timer: RefCell<SharedTimer>,
        pub runtime: RefCell<Runtime>,
        pub config: RefCell<Config>,
    }

    impl Default for TuxSplitContext {
        fn default() -> Self {
            // Lazy default: creates an empty Run; real initialization happens in
            // `TuxSplitContext::new_initialized()`. This allows GLib construction.
            let mut run = Run::new();
            run.set_game_name("Game");
            run.set_category_name("Category");
            let segment = livesplit_core::Segment::new("Segment 1");
            run.push_segment(segment);
            let timer = Timer::new(run).expect("timer");
            let shared = timer.into_shared();
            let runtime = Runtime::new(shared.clone());
            let config = Config::default();
            Self {
                timer: RefCell::new(shared),
                runtime: RefCell::new(runtime),
                config: RefCell::new(config),
            }
        }
    }

    #[glib::object_subclass]
    impl ObjectSubclass for TuxSplitContext {
        const NAME: &'static str = "TuxSplitContext";
        type Type = super::TuxSplitContext;
        type ParentType = glib::Object;
    }

    impl ObjectImpl for TuxSplitContext {
        fn signals() -> &'static [Signal] {
            static SIGNALS: OnceLock<Vec<Signal>> = OnceLock::new();
            SIGNALS.get_or_init(|| {
                vec![
                    // Emitted after the underlying Run is replaced or mutated
                    // (structure, times, metadata). Listeners should refresh
                    // any cached segment representations.
                    Signal::builder("run-changed").action().build(),
                ]
            })
        }
    }
}

glib::wrapper! {
    pub struct TuxSplitContext(ObjectSubclass<imp::TuxSplitContext>);
}

impl TuxSplitContext {
    /// Construct a new initialized global context.
    ///
    /// Panics if the timer or hotkey system cannot be created.
    fn init() -> Self {
        let mut config = load_config();
        let run = config.parse_run_or_default();

        let timer = Timer::new(run).expect("Failed to create timer");
        let shared_timer = timer.into_shared();

        let runtime = Runtime::new(shared_timer.clone());

        config.configure_timer(&mut shared_timer.write().unwrap());
        config.maybe_load_auto_splitter(&runtime);

        let Some(()) = config.create_hotkey_system(shared_timer.clone()) else {
            panic!("Could not load HotkeySystem");
        };

        let obj: Self = glib::Object::new();
        {
            let imp = obj.imp();
            imp.timer.replace(shared_timer);
            imp.runtime.replace(runtime);
            imp.config.replace(config);
        }

        obj
    }

    pub fn get_instance() -> Arc<Self> {
        thread_local! {
            static INSTANCE: OnceLock<Arc<TuxSplitContext>> = OnceLock::new();
        }
        INSTANCE.with(|instance| {
            instance
                .get_or_init(|| Arc::new(TuxSplitContext::init()))
                .clone()
        })
    }

    pub fn timer(&self) -> Arc<RwLock<Timer>> {
        self.imp().timer.borrow().clone()
    }

    pub fn get_run(&self) -> Run {
        self.timer().read().unwrap().run().clone()
    }

    pub fn config(&self) -> std::cell::Ref<Config> {
        self.imp().config.borrow()
    }

    pub fn config_mut(&self) -> Result<std::cell::RefMut<'_, Config>, std::cell::BorrowMutError> {
        self.imp().config.try_borrow_mut()
    }

    pub fn runtime(&self) -> std::cell::Ref<'_, Runtime> {
        self.imp().runtime.borrow()
    }

    pub fn emit_run_changed(&self) {
        self.emit_by_name::<()>("run-changed", &[]);
    }

    /// Replace the run (full set_run) and emit run-changed. Re-configures
    /// timer based on current config (useful if comparisons / settings depend
    /// on run contents).
    pub fn set_run(&self, new_run: Run) {
        let timer_arc = self.timer();
        {
            let mut timer = timer_arc.write().unwrap();
            let _ = timer.set_run(new_run);
            // Re-apply config in case it needs to reinitialize aspects of the timer.
            self.config().configure_timer(&mut timer);
        }
        self.emit_run_changed();
    }

    pub fn disable_hotkeys(&self) {
        if let Ok(mut cfg_write) = self.config_mut() {
            cfg_write.disable_hotkey_system();
        }
    }

    pub fn enable_hotkeys(&self) {
        if let Ok(mut cfg_write) = self.config_mut() {
            cfg_write.enable_hotkey_system();
        }
    }
}

pub fn build_ui(app: &Application) {
    let window: ApplicationWindow = ApplicationWindow::builder()
        .application(app)
        .title("TuxSplit")
        .build();

    let toolbar_view = ToolbarView::new();
    let header = TuxSplitHeader::new(&window);
    toolbar_view.add_top_bar(header.header());

    let mut timer_widget = TuxSplitTimer::new();
    timer_widget.start_refresh_loop();
    toolbar_view.set_content(Some(timer_widget.clamped()));

    window.set_content(Some(&toolbar_view));
    window.present();
}

pub fn shutdown() {
    info!("Shutting down TuxSplit");
    TuxSplitContext::get_instance()
        .config()
        .save(get_config_path().join("config.yaml"))
        .expect("Failed to save config on shutdown");
}

fn load_config() -> Config {
    let user_cfg = get_config_path().join("config.yaml");
    if user_cfg.is_file()
        && let Some(cfg) = Config::parse(&user_cfg)
    {
        debug!("Loaded user config {}", user_cfg.display());
        return cfg;
    }
    Config::default()
}

fn get_config_path() -> PathBuf {
    if let Ok(path_str) = env::var("TUXSPLIT_DATADIR") {
        PathBuf::from(&path_str)
    } else if let Ok(path_str) = env::var("XDG_CONFIG_HOME") {
        PathBuf::from(path_str).join("tuxsplit")
    } else if let Ok(home) = env::var("HOME") {
        let path = PathBuf::from(home).join(".config").join("tuxsplit");
        if !path.is_dir() {
            std::fs::create_dir_all(&path).expect("Failed to create config directory");
        }
        path
    } else {
        PathBuf::from("/tmp")
    }
}

#[allow(dead_code)]
fn register_gresource(resource_path: &Path) {
    if resource_path.exists() {
        let res = gio::Resource::load(resource_path).expect("Failed to load resource");
        gio::resources_register(&res);
        debug!("Registered GResource from {}", resource_path.display());
    }
}
