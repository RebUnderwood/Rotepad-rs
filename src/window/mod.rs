pub mod imp;

use gtk4::{
    gio, 
    glib, 
    glib::clone, 
    prelude::*, 
    subclass::prelude::*, 
    TextBuffer, 
    gio::ActionEntry,
    gio::MenuItem,
    AlertDialog,
    glib::Propagation,
};
use regex::Regex;
use std::sync::mpsc::Sender;
use rfd::FileDialog;
use std::path::PathBuf;
use std::fs::File;
use std::fs::OpenOptions;
use std::io;
use std::io::prelude::*;
use directories::ProjectDirs;
use toml;
use std::process::Command;
use std::env;


#[derive(Default)]
pub struct WindowData {
    pub wordcount: u32,
    pub path: Option<String>,
    pub contentunsaved: bool,
    pub autosavedisabled: bool,
}

glib::wrapper! {
    pub struct Window(ObjectSubclass<imp::Window>)
        @extends gtk4::ApplicationWindow, gtk4::Window, gtk4::Widget,
        @implements gio::ActionGroup, gio::ActionMap, gtk4::Accessible, gtk4::Buildable,
                    gtk4::ConstraintTarget, gtk4::Native, gtk4::Root, gtk4::ShortcutManager;
}

impl Window {
    pub fn new<P: IsA<gtk4::Application>>(app: &P) -> Self {
        glib::Object::builder().property("application", app).build()
    }

    pub fn setup(&self, upd_tx: Sender<u16>, console_tx: Sender<u16>) {
        self.imp().upd_tx.replace(upd_tx);
        self.imp().console_tx.replace(console_tx);

        self.imp().textarea.buffer().connect_changed(clone!(
            #[weak(rename_to = window)]
            self,
            move |_buff: &TextBuffer| {
                window.update_text_state();
            }),
        );

        self.connect_close_request(
            move |window: &Self| {
                window.close_save_guard();
                Propagation::Stop 
            },
        );

        let action_open_path = ActionEntry::builder("open_path")
            .parameter_type(Some(&str::static_variant_type()))
            .activate(move |window: &Self, _action, parameter| { 
                if let Some(path_var) = parameter {
                    if let Some(path) = path_var.str() {
                        window.open_save_guard(Some(PathBuf::from(path)));
                    }
                }
            })
            .build();

        let action_open = ActionEntry::builder("open")
            .activate(move |window: &Self, _action, _parameter| {
                window.open_save_guard(None);
            })
            .build();

        let action_save = ActionEntry::builder("save")
            .activate(move |window: &Self, _action, _parameter| {
                match window.path() {
                    Some(path) => {
                        match window.save_file(PathBuf::from(path), true) {
                            Ok(_) => {window.write_to_console("Saved!")},
                            Err(e) => {println!("{:?}", e)}
                        }
                    },
                    None => {
                        match FileDialog::new()
                            .set_title("Save As")
                            .add_filter("text", &["txt",])
                            .set_directory("/")
                            .save_file() 
                        {
                            Some(path) => {
                                match window.save_file(path, true) {
                                    Ok(_) => {
                                        window.set_contentunsaved(false);
                                        window.write_to_console("Saved!");
                                    },
                                    Err(e) => {println!("{:?}", e)}
                                }
                            },
                            None => {},
                        }
                    }
                }
            })
            .build();

        let action_save_as = ActionEntry::builder("save_as")
            .activate(move |window: &Self, _action, _parameter| {
                match FileDialog::new()
                    .set_title("Save As")
                    .add_filter("text", &["txt",])
                    .set_directory("/")
                    .save_file() 
                {
                    Some(path) => {
                        match window.save_file(path, true) {
                            Ok(_) => {
                                window.set_contentunsaved(false);
                                window.write_to_console("Saved!");
                            },
                            Err(e) => {println!("{:?}", e)}
                        }
                    },
                    None => {},
                }
            })
            .build();

        let action_toggle_autosave = ActionEntry::builder("toggle-autosave")
            .state((!self.autosavedisabled()).to_variant())
            .activate(move |window: &Self, action, _parameter| {
                let state = action.state().unwrap();
                let action_state: bool = state.get().unwrap();
                let new_state = !action_state;
                action.set_state(&new_state.to_variant());
                window.set_autosavedisabled(!new_state);
            })
            .build();

        let action_new = ActionEntry::builder("new")
            .activate(move |window: &Self, _action, _parameter| { 
                window.new_save_guard();
            })
            .build();

        let action_toggle_fullscreen = ActionEntry::builder("toggle-fullscreen")
            .state((false).to_variant())
            .activate(move |window: &Self, action, _parameter| {
                let state = action.state().unwrap();
                let action_state: bool = state.get().unwrap();
                let new_state = !action_state;
                action.set_state(&new_state.to_variant());
                if new_state {
                    window.fullscreen();
                    window.add_css_class("fullscreen");
                    window.imp().right_margin.set_hexpand(true);
                    window.imp().left_margin.set_hexpand(true);
                    window.imp().textarea.set_hexpand(false);
                } else {
                    window.unfullscreen();
                    window.remove_css_class("fullscreen");
                    window.imp().right_margin.set_hexpand(false);
                    window.imp().left_margin.set_hexpand(false);
                    window.imp().textarea.set_hexpand(true);
                }
            })
            .build();

        let action_toggle_status_bar = ActionEntry::builder("toggle-status-bar")
            .state((true).to_variant())
            .activate(move |window: &Self, action, _parameter| {
                let state = action.state().unwrap();
                let action_state: bool = state.get().unwrap();
                let new_state = !action_state;
                action.set_state(&new_state.to_variant());
                if new_state {
                    window.imp().status_bar.set_visible(true);
                } else {
                    window.imp().status_bar.set_visible(false);
                }
            })
            .build();

        let action_toggle_wordcount_display = ActionEntry::builder("toggle-wordcount-display")
            .state((true).to_variant())
            .activate(move |window: &Self, action, _parameter| {
                let state = action.state().unwrap();
                let action_state: bool = state.get().unwrap();
                let new_state = !action_state;
                action.set_state(&new_state.to_variant());
                if new_state {
                    window.imp().wordcount_display.set_visible(true);
                } else {
                    window.imp().wordcount_display.set_visible(false);
                }
            })
            .build();

        let action_new_window = ActionEntry::builder("new-window")
            .activate(move |window: &Self, _action, _parameter| {
                match window.new_window(None) {
                    Ok(_) => {},
                    Err(e) => {println!("{:?}", e)}
                }
            })
            .build();

        let action_open_in_new_window = ActionEntry::builder("open-in-new-window")
            .activate(move |window: &Self, _action, _parameter| {
                match FileDialog::new()
                    .set_title("Open")
                    .add_filter("text", &["txt",])
                    .set_directory("/")
                    .pick_file() 
                {
                    Some(path) => {
                        match window.new_window(Some(path)) {
                            Ok(_) => {},
                            Err(e) => {println!("{:?}", e)}
                        }
                    },
                    None => {},
                }
            })
            .build();

        let action_undo = ActionEntry::builder("undo")
            .activate(move |window: &Self, _action, _parameter| {
                window.imp().textarea.buffer().undo();
            })
            .build();

        let action_redo = ActionEntry::builder("redo")
            .activate(move |window: &Self, _action, _parameter| {
                window.imp().textarea.buffer().redo();
            })
            .build();

        let action_cut = ActionEntry::builder("cut")
            .activate(move |window: &Self, _action, _parameter| {
                window.imp().textarea.emit_cut_clipboard();
            })
            .build();

        let action_copy = ActionEntry::builder("copy")
            .activate(move |window: &Self, _action, _parameter| {
                window.imp().textarea.emit_copy_clipboard();
            })
            .build();

        let action_paste = ActionEntry::builder("paste")
            .activate(move |window: &Self, _action, _parameter| {
                window.imp().textarea.emit_paste_clipboard();
            })
            .build();

        let select_all = ActionEntry::builder("select-all")
            .activate(move |window: &Self, _action, _parameter| {
                window.imp().textarea.emit_select_all(true);
            })
            .build();

        self.add_action_entries([
            action_open_path,
            action_open,
            action_save,
            action_save_as,
            action_toggle_autosave,
            action_new,
            action_toggle_fullscreen,
            action_toggle_status_bar,
            action_toggle_wordcount_display,
            action_new_window,
            action_open_in_new_window,
            action_undo,
            action_redo,
            action_cut,
            action_copy,
            action_paste,
            select_all
        ]);

        self.generate_recent_files_menu();

        self.imp().textarea.grab_focus();
    }

    fn update_text_state(&self) {
        self.set_contentunsaved(true);
        match self.imp().upd_tx.try_borrow() {
            Ok(sender) => {
                // 350 milliseconds (7 * 50)
                sender.send(7).unwrap();
            },
            Err(_) => {},
        }
    }

    pub fn count_words(&self) {
        let wordcount_regex: regex::Regex = Regex::new("[\\w-]+").unwrap();
        let wordcount: u32 = wordcount_regex.find_iter(&self.content()).count() as u32;
        self.set_wordcount(wordcount);
        self.imp().wordcount_display.get().set_label(&format!("{} words", thousandify(self.wordcount())));
    }

    pub fn autosave(&self) {
        if self.autosavedisabled() {
            return;
        }
        if !self.contentunsaved(){
            return;
        }
        match self.path() {
            Some(path) => {
                match self.save_file(PathBuf::from(path), false) {
                    Ok(_) => {
                        self.set_contentunsaved(false);
                        self.write_to_console("Autosaved!")
                    },
                    Err(e) => {println!("{:?}", e)}
                }
            },
            None => {},
        }
    }

    fn write_to_console(&self, message: &str) {
        let console = self.imp().console_display.get();
        console.set_label("");
        console.remove_css_class("hide");
        console.set_label(message);

        match self.imp().console_tx.try_borrow() {
            Ok(sender) => {
                // 1 second (20 * 50 milliseconds)
                sender.send(20).unwrap();
            },
            Err(_) => {},
        }
    }

    pub fn fade_console(&self) {
        let console = self.imp().console_display.get();
        console.add_css_class("hide");
    }

    fn generate_recent_files_menu(&self) {
        match self.get_config_field("recent_files") {
            Ok(val_opt) => {
                match val_opt {
                    Some(arr) => {
                        if let toml::Value::Array(filepaths) = arr {
                            let menu = self.imp().recent_files_menu.get();
                            menu.remove_all();
                            for i in 0..filepaths.len() {
                                if let toml::Value::String(path) = &filepaths[i] {
                                    let item = MenuItem::new(Some(&path), None);
                                    item.set_action_and_target_value(Some("win.open_path"), Some(&path.to_variant()));
                                    menu.insert_item(i as i32, &item);
                                }
                            }
                        }
                    },
                    None => {
                        let menu = self.imp().recent_files_menu.get();
                        let item = MenuItem::new(Some("No recent files."), None);
                        menu.insert_item(0, &item);
                    }
                }
            },
            Err(e) => {println!("{:?}", e)}
        }
    }

    fn new_save_guard(&self) {
        if self.contentunsaved() == true {
            let alert = AlertDialog::builder()
                .modal(true)
                .message("Warning!")
                .detail("You have unsaved data. It will be lost if you open a new file. Save?")
                .buttons(["Yes", "No"])
                .build();
            alert.choose(Some(self), None::<&gtk4::gio::Cancellable>, clone!(
                #[weak(rename_to = window)]
                self,
                move |r| {
                    if let Ok(button) = r {
                        if button == 0 {
                            match window.path() {
                                Some(s_path) => {
                                    match window.save_file(PathBuf::from(s_path), true) {
                                        Ok(_) => {
                                            window.set_contentunsaved(false);
                                            window.write_to_console("Saved!");
                                        },
                                        Err(e) => {println!("{:?}", e)}
                                    }
                                },
                                None => {
                                    match FileDialog::new()
                                        .set_title("Save As")
                                        .add_filter("text", &["txt",])
                                        .set_directory("/")
                                        .save_file() 
                                    {
                                        Some(path) => {
                                            match window.save_file(path, true) {
                                                Ok(_) => {
                                                    window.set_contentunsaved(false);
                                                    window.write_to_console("Saved!");
                                                },
                                                Err(e) => {println!("{:?}", e)}
                                            }
                                        },
                                        None => {},
                                    }
                                }
                            }
                        }
                    }
                    window.imp().textarea.buffer().set_text("");
                    window.clear_content();
                    window.set_contentunsaved(false);
                    window.set_path(None::<String>);
                }
            ));
        } else {
            self.imp().textarea.buffer().set_text("");
            self.clear_content();
            self.set_contentunsaved(false);
            self.set_path(None::<String>);
        }
    }

    fn close_save_guard(&self) {
        if self.contentunsaved() == true {
            let alert = AlertDialog::builder()
                .modal(true)
                .message("Warning!")
                .detail("You have unsaved data. It will be lost if you open a new file. Save?")
                .buttons(["Yes", "No"])
                .build();
            alert.choose(Some(self), None::<&gtk4::gio::Cancellable>, clone!(
                #[weak(rename_to = window)]
                self,
                move |r| {
                    if let Ok(button) = r {
                        if button == 0 {
                            match window.path() {
                                Some(s_path) => {
                                    match window.save_file(PathBuf::from(s_path), true) {
                                        Ok(_) => {
                                            window.set_contentunsaved(false);
                                            window.write_to_console("Saved!");
                                        },
                                        Err(e) => {println!("{:?}", e)}
                                    }
                                },
                                None => {
                                    match FileDialog::new()
                                        .set_title("Save As")
                                        .add_filter("text", &["txt",])
                                        .set_directory("/")
                                        .save_file() 
                                    {
                                        Some(path) => {
                                            match window.save_file(path, true) {
                                                Ok(_) => {
                                                    window.set_contentunsaved(false);
                                                    window.write_to_console("Saved!");
                                                },
                                                Err(e) => {println!("{:?}", e)}
                                            }
                                        },
                                        None => {},
                                    }
                                }
                            }
                        }
                    }
                    window.destroy();
                }
            ));
        } else {
            self.destroy();
        }
    }

    fn open_save_guard(&self, path: Option<PathBuf>) {
        if self.contentunsaved() == true {
            let alert = AlertDialog::builder()
                .modal(true)
                .message("Warning!")
                .detail("You have unsaved data. It will be lost if you open a new file. Save?")
                .buttons(["Yes", "No"])
                .build();
            alert.choose(Some(self), None::<&gtk4::gio::Cancellable>, clone!(
                #[weak(rename_to = window)]
                self,
                move |r| {
                    if let Ok(button) = r {
                        if button == 0 {
                            match window.path() {
                                Some(s_path) => {
                                    match window.save_file(PathBuf::from(s_path), true) {
                                        Ok(_) => {
                                            window.set_contentunsaved(false);
                                            window.write_to_console("Saved!");
                                        },
                                        Err(e) => {println!("{:?}", e)}
                                    }
                                },
                                None => {
                                    match FileDialog::new()
                                        .set_title("Save As")
                                        .add_filter("text", &["txt",])
                                        .set_directory("/")
                                        .save_file() 
                                    {
                                        Some(path) => {
                                            match window.save_file(path, true) {
                                                Ok(_) => {
                                                    window.set_contentunsaved(false);
                                                    window.write_to_console("Saved!");
                                                },
                                                Err(e) => {println!("{:?}", e)}
                                            }
                                        },
                                        None => {},
                                    }
                                }
                            }
                        }
                    }
                    match path {
                        Some(p) => {
                            match window.open_file(p) {
                                Ok(_) => {window.set_contentunsaved(false);},
                                Err(e) => {println!("{:?}", e)}
                            }
                        }
                        None => {
                            match FileDialog::new()
                                .set_title("Open")
                                .add_filter("text", &["txt",])
                                .set_directory("/")
                                .pick_file() 
                            {
                                Some(path) => {
                                    match window.open_file(path) {
                                        Ok(_) => {window.set_contentunsaved(false);},
                                        Err(e) => {println!("{:?}", e)}
                                    }
                                },
                                None => {},
                            }
                        }
                    }
                }
            ));
        } else {
            match path {
                Some(p) => {
                    match self.open_file(p) {
                        Ok(_) => {self.set_contentunsaved(false);},
                        Err(e) => {println!("{:?}", e)}
                    }
                }
                None => {
                    match FileDialog::new()
                        .set_title("Open")
                        .add_filter("text", &["txt",])
                        .set_directory("/")
                        .pick_file() 
                    {
                        Some(path) => {
                            match self.open_file(path) {
                                Ok(_) => {self.set_contentunsaved(false);},
                                Err(e) => {println!("{:?}", e)}
                            }
                        },
                        None => {},
                    }
                }
            }
        }
    }

    pub fn open_file(&self, path: PathBuf) -> Result<(), io::Error> {
        let mut file = File::open(&path)?;
        match path.into_os_string().to_str() {
            Some(path_str) => {
                self.set_path(Some(path_str));
                match self.get_config_field("recent_files") {
                    Ok(val_opt) => {
                        match val_opt {
                            Some(arr) => {
                                if let toml::Value::Array( mut filepaths) = arr {
                                    filepaths.retain(|x| {
                                        if let toml::Value::String(p) = x {
                                            return *p != path_str.to_string();
                                        } else {
                                            return true;
                                        }
                                    });
                                    filepaths.insert(0, toml::Value::String(path_str.to_string()));
                                    filepaths.truncate(15);
                                    match self.set_config_field("recent_files", toml::Value::Array(filepaths)) {
                                        Ok(_) => {self.generate_recent_files_menu()},
                                        Err(e) => {println!("{:?}", e)}
                                    }
                                }
                            },
                            None => {
                                let filepaths = vec![toml::Value::String(path_str.to_string())];
                                match self.set_config_field("recent_files", toml::Value::Array(filepaths)) {
                                    Ok(_) => {self.generate_recent_files_menu()},
                                    Err(e) => {println!("{:?}", e)}
                                }
                            }
                        }
                    },
                    Err(e) => {println!("{:?}", e)}
                }
            },
            None => {
                return Err(io::Error::new(io::ErrorKind::Other, "File path was not valid."))
            }
        }
        let mut contents: String = String::new();
        file.read_to_string(&mut contents)?;
        self.set_content(contents);


        Ok(())
    }

    fn save_file(&self, path: PathBuf, add_to_recent: bool) -> Result<(), io::Error> {
        let mut file = File::create(&path)?;
        match path.into_os_string().to_str() {
            Some(path_str) => {
                self.set_path(Some(path_str));
                if add_to_recent {
                    match self.get_config_field("recent_files") {
                        Ok(val_opt) => {
                            match val_opt {
                                Some(arr) => {
                                    if let toml::Value::Array( mut filepaths) = arr {
                                        filepaths.retain(|x| {
                                            if let toml::Value::String(p) = x {
                                                return *p != path_str.to_string();
                                            } else {
                                                return true;
                                            }
                                        });
                                        filepaths.insert(0, toml::Value::String(path_str.to_string()));
                                        filepaths.truncate(15);
                                        match self.set_config_field("recent_files", toml::Value::Array(filepaths)) {
                                            Ok(_) => {self.generate_recent_files_menu()},
                                            Err(e) => {println!("{:?}", e)}
                                        }
                                    }
                                },
                                None => {
                                    let filepaths = vec![toml::Value::String(path_str.to_string())];
                                    match self.set_config_field("recent_files", toml::Value::Array(filepaths)) {
                                        Ok(_) => {self.generate_recent_files_menu()},
                                        Err(e) => {println!("{:?}", e)}
                                    }
                                }
                            }
                        },
                        Err(e) => {println!("{:?}", e)}
                    }
                }
            },
            None => {
                return Err(io::Error::new(io::ErrorKind::Other, "File path was not valid."))
            }
        }
        file.write_all(self.content().as_bytes())?;
        Ok(())
    }

    fn new_window(&self, path: Option<PathBuf>) -> io::Result<()> {
        let exe_path = env::current_exe()?;
        if let Some(path_str) = exe_path.into_os_string().to_str() {
            if cfg!(target_os = "windows") {
                let mut cmd = Command::new("path_str");
                match path {
                    Some(o_path_buf) => {
                        if let Some(o_path_str) = o_path_buf.into_os_string().to_str() {
                            cmd.arg(o_path_str);
                        }
                    },
                    None => {}
                }
                cmd.spawn()?
            } else {
                let mut cmd = Command::new(path_str);
                match path {
                    Some(o_path_buf) => {
                        if let Some(o_path_str) = o_path_buf.into_os_string().to_str() {
                            cmd.arg(o_path_str);
                        }
                    },
                    None => {}
                }
                cmd.spawn()?
            };
        }
        Ok(())
    }

    fn get_config_field(&self, field_name: &str) -> Result<Option<toml::Value>, io::Error> {
        if let Some(proj_dirs) = ProjectDirs::from("com", "Reb",  "Rotepad") {
            let path = proj_dirs.config_dir();
            if path.exists() {
                let mut file = File::open(&path)?;
                let mut contents: String = String::new();
                file.read_to_string(&mut contents)?;
                let config = contents.parse::<toml::Table>().unwrap();
                if config.contains_key(field_name) {
                    let out = config[field_name].clone();
                    return Ok(Some(out));
                } else {
                    return Ok(None);
                }
            } else {
                return Ok(None);
            }
        } else {
            return Err(io::Error::new(io::ErrorKind::Other, "Could not find Home dir."))
        }
    }

    fn set_config_field(&self, field_name: &str, val: toml::Value) -> Result<(), io::Error> {
        if let Some(proj_dirs) = ProjectDirs::from("com", "Reb",  "Rotepad") {
            let path = proj_dirs.config_dir();
            if path.exists() {
                let mut file = OpenOptions::new()
                    .read(true)
                    .write(true)
                    .open(&path)?;
                let mut contents: String = String::new();
                file.read_to_string(&mut contents)?;
                let mut config = contents.parse::<toml::Table>().unwrap();
                config.insert(field_name.to_string(), val);
                file.rewind()?;
                file.set_len(0)?;
                file.write_all(toml::to_string(&config).unwrap().as_bytes())?;
                return Ok(());
            } else {
                let mut file = File::create(&path)?;
                let mut config = toml::Table::new();
                config.insert(field_name.to_string(), val);
                file.write_all(toml::to_string(&config).unwrap().as_bytes())?;
                return Ok(());
            }
        } else {
            return Err(io::Error::new(io::ErrorKind::Other, "Could not find Home dir."))
        }
    }

    pub fn test_write_output(&self, s: String) {
        self.imp().textarea.buffer().set_text(&s);
    }

    fn content(&self) -> String {
        let buff = self.imp().textarea.buffer();
        buff.text(&buff.start_iter(), &buff.end_iter(), true).to_string()
    }

    fn set_content(&self, content: String) {
        self.imp().textarea.buffer().set_text(&content);
    }

    fn clear_content(&self) {
        self.imp().textarea.buffer().set_text("");
    }
}

fn thousandify(num: u32) -> String {
    let num_str = num.to_string();
    let mut chars: Vec<char> = num_str.chars().collect();
    chars.reverse();
    let mut commad_vec: Vec<char> = vec![];
    for i in 0..chars.len() {
        commad_vec.push(chars[i]);
        if i != 0 && i+1 != chars.len() && (i+1) % 3 == 0 {
            commad_vec.push(',');
        }
        
    }
    commad_vec.reverse();
    commad_vec.into_iter().collect::<String>()
}