mod window;

use gtk4::prelude::*;
use gtk4::{Application, gio, gdk, gio::ApplicationFlags};
use window::Window;
use std::time::Duration;
use std::thread::sleep;
use std::sync::{Arc, Mutex};
use std::thread;
use std::sync::mpsc::channel;
use std::env;
use std::path::PathBuf;

static WINDOW_WRAPPER: Mutex<Option<WindowWrapper>> = Mutex::new(None);

fn main()  {
    // Register and include resources
    gio::resources_register_include!("compiled.gresource")
        .expect("Failed to register resources.");

    // Create a new application
    let app = Application::builder()
        .application_id("org.gtk_rs.Rotepad")
        .flags(ApplicationFlags::HANDLES_OPEN)
        .build();

    app.connect_startup(|_| load_css());
    app.connect_activate(build_ui);
    app.connect_open(|app, files, _hint| {
        app.activate(); 
        let w_opt = WINDOW_WRAPPER.lock().unwrap();
        if let Some(wrapper) = &*w_opt {
            if let Some(path) = files[0].path() {
                wrapper.open_file(path);
            }
        }
    });

    app.set_accels_for_action("win.open", &["<Ctrl>O"]);
    app.set_accels_for_action("win.save", &["<Ctrl>S"]);
    app.set_accels_for_action("win.save_as", &["<Ctrl><Shift>S"]);
    app.set_accels_for_action("win.new", &["<Ctrl>N"]);
    app.set_accels_for_action("win.new-window", &["<Ctrl><Shift>N"]);
    app.set_accels_for_action("win.open-in-new-window", &["<Ctrl><Shift>O"]);
    app.set_accels_for_action("win.toggle-fullscreen", &["F11"]);

    app.set_accels_for_action("win.undo", &["<Ctrl>Z"]);
    app.set_accels_for_action("win.redo", &["<Ctrl>Y", "<Ctrl><Shift>Z"]);
    app.set_accels_for_action("win.cut", &["<Ctrl>X"]);
    app.set_accels_for_action("win.copy", &["<Ctrl>C"]);
    app.set_accels_for_action("win.paste", &["<Ctrl>V"]);

    app.set_accels_for_action("win.select-all", &["<Ctrl>A"]);

    

    // Run the application
    app.run();
}

fn load_css() {
    // Load the CSS file and add it to the provider
    let provider = gtk4::CssProvider::new();
    provider.load_from_string(include_str!("style.css"));

    // Add the provider to the default screen
    gtk4::style_context_add_provider_for_display(
        &gdk::Display::default().expect("Could not connect to a display."),
        &provider,
        gtk4::STYLE_PROVIDER_PRIORITY_APPLICATION,
    );
}

fn build_ui(app: &Application) {
    let window = Window::new(app);
    #[allow(unused_variables)]
    let (upd_tx, upd_rx) = channel();
    let (console_tx, console_rx) = channel();
    window.setup(upd_tx, console_tx);
    
    
    window.present();
    let wrapper = WindowWrapper::new(window);
    let mut w_opt = WINDOW_WRAPPER.lock().unwrap();
    *w_opt = Some(wrapper);
    thread::spawn(
        move || {
            let mut countdown = 0;
            let mut console_countdown = 0;
            loop {
                sleep(Duration::from_millis(50));
                match upd_rx.try_recv() {
                    Ok(reset_val) => {
                        countdown = reset_val;
                    },
                    Err(_) => {},
                }
                match console_rx.try_recv() {
                    Ok(reset_val) => {
                        console_countdown = reset_val;
                    },
                    Err(_) => {},
                }
                if countdown > 0 {
                    countdown -= 1;
                    if countdown == 0 {
                        let w_opt = WINDOW_WRAPPER.lock().unwrap();
                        if let Some(wrapper) = &*w_opt {
                            wrapper.count_words();
                            wrapper.autosave();
                        }
                    }
                }
                if console_countdown > 0 {
                    console_countdown -= 1;
                    if console_countdown == 0 {
                        let w_opt = WINDOW_WRAPPER.lock().unwrap();
                        if let Some(wrapper) = &*w_opt {
                            wrapper.fade_console();
                        }
                    }
                }
            }
        },
    );
}


struct WindowWrapper {
    window: Arc<Mutex<Window>>,
}

impl WindowWrapper {
    pub fn new(window: Window) -> Self {
        WindowWrapper {
            window: Arc::new(Mutex::new(window)),
        }
    }
    pub fn count_words(&self) {
        let _ = &self.window.lock().unwrap().count_words();
    }
    pub fn autosave(&self) {
        let _ = &self.window.lock().unwrap().autosave();
    }
    pub fn fade_console(&self) {
        let _ = &self.window.lock().unwrap().fade_console();
    }
    pub fn open_file(&self, path: PathBuf) {
        let _ = &self.window.lock().unwrap().open_file(path);
    }
}

unsafe impl Send for WindowWrapper {}
unsafe impl Sync for WindowWrapper {}