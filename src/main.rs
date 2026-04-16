

mod window;

use gtk4::prelude::*;
use gtk4::{Application, gio, glib, gdk};
use window::Window;
use std::time::Duration;
use std::thread::sleep;
use std::sync::{Arc, Mutex};
use std::thread;
use std::sync::mpsc::channel;

fn main() -> glib::ExitCode {
    // Register and include resources
    gio::resources_register_include!("compiled.gresource")
        .expect("Failed to register resources.");

    // Create a new application
    let app = Application::builder()
        .application_id("org.gtk_rs.Rotepad")
        .build();

    // Connect to "activate" signal of `app`
    app.connect_startup(|_| load_css());
    app.connect_activate(build_ui);

    app.set_accels_for_action("win.open", &["<Ctrl>O"]);
    app.set_accels_for_action("win.save", &["<Ctrl>S"]);
    app.set_accels_for_action("win.save_as", &["<Ctrl><Shift>S"]);

    // Run the application
    app.run()
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
                        wrapper.count_words();
                        wrapper.autosave();
                    }
                }
                if console_countdown > 0 {
                    console_countdown -= 1;
                    if console_countdown == 0 {
                        wrapper.fade_console();
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
}

unsafe impl Send for WindowWrapper {}
unsafe impl Sync for WindowWrapper {}