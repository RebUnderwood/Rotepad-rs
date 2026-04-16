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
    //gio::Menu,
    MenuButton
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


#[derive(Default)]
pub struct WindowData {
    pub content: String,
    pub wordcount: u32,
    pub path: Option<String>,
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
            move |buff: &TextBuffer| {
                window.update_text_state(buff.text(&buff.start_iter(), &buff.end_iter(), true).to_string());
            }),
        );

        self.imp().file_menu_button.connect_notify_local(Some("active"), clone!(
            #[weak(rename_to = window)]
            self,
            move |button, _| {
                if button.is_active() {
                    match window.get_config_field("recent_files") {
                        Ok(val_opt) => {
                            match val_opt {
                                Some(arr) => {
                                    if let toml::Value::Array( mut filepaths) = arr {
                                        let menu = window.imp().recent_files_menu.get();
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
                                    let menu = window.imp().recent_files_menu.get();
                                    let item = MenuItem::new(Some("No recent files."), None);
                                    menu.insert_item(0, &item);
                                }
                            }
                        },
                        Err(e) => {println!("{:?}", e)}
                    }
                }
            }

        ));

        let action_open_path = ActionEntry::builder("open_path")
            .parameter_type(Some(&str::static_variant_type()))
            .activate(move |window: &Self, _action, parameter| { 
                if let Some(path_var) = parameter {
                    if let Some(path) = path_var.str() {
                        match window.open_file(PathBuf::from(path)) {
                            Ok(_) => {},
                            Err(e) => {println!("{:?}", e)}
                        }
                    }
                    
                }
            })
            .build();

        let action_open = ActionEntry::builder("open")
            .activate(move |window: &Self, _action, parameter| {
                println!("{:?}", parameter);
                match FileDialog::new()
                    .set_title("Open")
                    .add_filter("text", &["txt",])
                    .set_directory("/")
                    .pick_file() 
                {
                    Some(path) => {
                        match window.open_file(path) {
                            Ok(_) => {},
                            Err(e) => {println!("{:?}", e)}
                        }
                    },
                    None => {},
                }
            })
            .build();

        let action_save = ActionEntry::builder("save")
            .activate(move |window: &Self, _action, _parameter| {
                match window.path() {

                    Some(path) => {
                        match window.save_file(PathBuf::from(path)) {
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
                                match window.save_file(path) {
                                    Ok(_) => {window.write_to_console("Saved!")},
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
                        match window.save_file(path) {
                            Ok(_) => {window.write_to_console("Saved!")},
                            Err(e) => {println!("{:?}", e)}
                        }
                    },
                    None => {},
                }
            })
            .build();

        // let action_generate_recent_files_menu = ActionEntry::builder("generate_recent_files_menu")
        //     .activate(move |window: &Self, _action, _parameter| {
        //         println!("sadgsgsfgdfgdshsfghdfghf");
        //        match window.get_config_field("recent_files") {
        //             Ok(val_opt) => {
        //                 match val_opt {
        //                     Some(arr) => {
        //                         if let toml::Value::Array( mut filepaths) = arr {
        //                             let menu = window.imp().recent_files_menu.get();
        //                             menu.remove_all();
        //                             for i in 0..filepaths.len() {
        //                                 if let toml::Value::String(path) = &filepaths[i] {
        //                                     let item = MenuItem::new(Some(&path), None);
        //                                     menu.insert_item(i as i32, &item);
        //                                 }
        //                             }
        //                         }
        //                     },
        //                     None => {
        //                         let menu = window.imp().recent_files_menu.get();
        //                         let item = MenuItem::new(Some("No recent files."), None);
        //                         menu.insert_item(0, &item);
        //                     }
        //                 }
        //             },
        //             Err(e) => {println!("{:?}", e)}
        //         }
        //     })
        //     .build();

        self.add_action_entries([
            action_open_path,
            action_open,
            action_save,
            action_save_as,
            //action_generate_recent_files_menu,
        ]);
    }

    fn update_text_state(&self, cont: String) {
        self.set_content(cont);
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
        match self.path() {
            Some(path) => {
                match self.save_file(PathBuf::from(path)) {
                    Ok(_) => {self.write_to_console("Autosaved!")},
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

    fn open_file(&self, path: PathBuf) -> Result<(), io::Error> {
        let mut file = File::open(&path)?;
        match path.into_os_string().to_str() {
            Some(path_str) => {
                self.set_path(path_str);
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
                                        Ok(_) => {},
                                        Err(e) => {println!("{:?}", e)}
                                    }
                                }
                            },
                            None => {
                                let filepaths = vec![toml::Value::String(path_str.to_string())];
                                match self.set_config_field("recent_files", toml::Value::Array(filepaths)) {
                                    Ok(_) => {},
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
        self.imp().textarea.buffer().set_text(&contents);
        self.set_content(contents);


        Ok(())
    }

    fn save_file(&self, path: PathBuf) -> Result<(), io::Error> {
        let mut file = File::create(&path)?;
        match path.into_os_string().to_str() {
            Some(path_str) => {
                self.set_path(path_str);
            },
            None => {
                return Err(io::Error::new(io::ErrorKind::Other, "File path was not valid."))
            }
        }
        file.write_all(self.content().as_bytes())?;
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