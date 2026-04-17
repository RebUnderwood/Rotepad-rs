use gtk4::{
    glib, 
    glib::Properties, 
    prelude::*, 
    subclass::prelude::*, 
    CompositeTemplate,
    glib::subclass::InitializingObject,
    TextView,
    Label,
    Box,
    gio::Menu,
};
use std::cell::RefCell;
use std::sync::{Arc, Mutex};
use std::sync::mpsc::{channel, Sender};
use crate::window::WindowData;


// Object holding the state
#[derive(CompositeTemplate, Properties)]
#[template(resource = "/org/gtk_rs/Rotepad-rs/window.ui")]
#[properties(wrapper_type = super::Window)]
pub struct Window {
    #[property(name = "wordcount", get, set, type = u32, member = wordcount)]
    #[property(name = "path", get, set,nullable, type = Option<String>, member = path)]
    #[property(name = "contentunsaved", get, set, type = bool, member = contentunsaved)]
    #[property(name = "autosavedisabled", get, set, type = bool, member = autosavedisabled)]
    pub data: Arc<Mutex<WindowData>>,
    #[template_child]
    pub textarea: TemplateChild<TextView>,
    #[template_child]
    pub status_bar: TemplateChild<Box>,
    #[template_child]
    pub wordcount_display: TemplateChild<Label>,
    #[template_child]
    pub zoom_display: TemplateChild<Label>,
    #[template_child]
    pub console_display: TemplateChild<Label>,
    #[template_child]
    pub recent_files_menu: TemplateChild<Menu>,
    #[template_child]
    pub left_margin: TemplateChild<Box>,
    #[template_child]
    pub right_margin: TemplateChild<Box>,
    pub upd_tx: RefCell<Sender<u16>>,
    pub console_tx: RefCell<Sender<u16>>,
}

impl Default for Window {
    fn default() -> Self {
        let (upd_tx, _) = channel();
        let (console_tx, _) = channel();
        Window {
            upd_tx: RefCell::new(upd_tx),
            console_tx: RefCell::new(console_tx),
            data: Default::default(),
            textarea: Default::default(),
            status_bar: Default::default(),
            wordcount_display: Default::default(),
            zoom_display: Default::default(),
            console_display: Default::default(),
            recent_files_menu: Default::default(),
            left_margin: Default::default(),
            right_margin: Default::default(),
        }
    }
}


// The central trait for subclassing a GObject
#[glib::object_subclass]
impl ObjectSubclass for Window {
    // `NAME` needs to match `class` attribute of template
    const NAME: &'static str = "RotepadWindow";
    type Type = super::Window;
    type ParentType = gtk4::ApplicationWindow;

    fn class_init(klass: &mut Self::Class) {
        klass.bind_template();
    }

    fn instance_init(obj: &InitializingObject<Self>) {
        obj.init_template();
    }
}

#[glib::derived_properties]
impl ObjectImpl for Window {
    fn constructed(&self) {
        self.parent_constructed();
    }
}

impl WidgetImpl for Window {}
impl WindowImpl for Window {}
impl ApplicationWindowImpl for Window {}