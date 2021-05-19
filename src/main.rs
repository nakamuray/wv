use gio::prelude::*;
use gtk::prelude::*;
use std::cell::RefCell;
use std::env;
use std::rc::Rc;

use gtk::Application;

mod settings;
mod viewer;
mod window;

fn main() {
    let settings = Rc::new(RefCell::new(settings::load_settings()));

    let app = Application::new(Some("org.u7fa9.wv"), gio::ApplicationFlags::HANDLES_OPEN)
        .expect("failed to initialize GTK application");
    app.connect_startup(|_app| {
        let screen = gdk::Screen::get_default().expect("can't get display");
        let provider = gtk::CssProvider::new();
        provider
            .load_from_data(include_bytes!("css/style.css"))
            .unwrap();
        gtk::StyleContext::add_provider_for_screen(
            &screen,
            &provider,
            gtk::STYLE_PROVIDER_PRIORITY_USER,
        );
    });
    app.connect_open(glib::clone!(@strong settings => move |app, files, _hints| {
        let window_settings = &settings.borrow().window;
        for f in files {
            let win = window::Window::new(&app);
            win.widget.set_default_size(window_settings.width, window_settings.height);
            win.widget.show_all();
            win.widget.connect_size_allocate(glib::clone!(@strong settings => move |win, _rect| {
                let (width, height) = win.get_size();
                (*settings.borrow_mut()).window.height = height;
                (*settings.borrow_mut()).window.width = width;
            }));
            win.load_uri(&f.get_uri());
        }
    }));
    app.connect_activate(glib::clone!(@strong settings => move |app| {
        let window_settings = &settings.borrow().window;
        let win = window::Window::new(&app);
        win.widget.set_default_size(window_settings.width, window_settings.height);
        win.widget.show_all();
        win.widget.connect_size_allocate(glib::clone!(@strong settings => move |win, _rect| {
            let (width, height) = win.get_size();
            (*settings.borrow_mut()).window.height = height;
            (*settings.borrow_mut()).window.width = width;
        }));
        win.load_uri("about:blank");
    }));
    app.connect_shutdown(glib::clone!(@strong settings => move |_app| {
        settings::save_settings(&settings.borrow());
    }));
    app.run(&env::args().collect::<Vec<_>>());
}
