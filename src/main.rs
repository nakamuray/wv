use gtk4 as gtk;

use gtk::glib::clone;
use gtk::prelude::*;
use gtk::{gdk, gio};
use std::cell::RefCell;
use std::rc::Rc;

use gtk::Application;

mod faviconheaderbar;
mod settings;
mod viewer;
mod window;

fn main() {
    let settings = Rc::new(RefCell::new(settings::load_settings()));

    let app = Application::new(Some("org.u7fa9.wv"), gio::ApplicationFlags::HANDLES_OPEN);
    app.connect_startup(|_app| {
        let display = gdk::Display::default().expect("can't get display");
        let provider = gtk::CssProvider::new();
        provider.load_from_data(include_bytes!("css/style.css"));
        gtk::StyleContext::add_provider_for_display(
            &display,
            &provider,
            gtk::STYLE_PROVIDER_PRIORITY_USER,
        );
    });
    app.connect_open(clone!(@strong settings => move |app, files, _hints| {
        for f in files {
            let win = window::Window::new(&app, settings.clone());
            win.widget.show();
            win.load_uri(&f.uri());
        }
    }));
    app.connect_activate(clone!(@strong settings => move |app| {
        let win = window::Window::new(&app, settings.clone());
        win.widget.show();
        win.load_uri("about:blank");
    }));
    app.connect_shutdown(clone!(@strong settings => move |_app| {
        settings::save_settings(&settings.borrow());
    }));
    app.run();
}
