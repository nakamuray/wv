use gio::prelude::*;
use gtk::prelude::*;
use std::env;

use gtk::Application;

mod viewer;
mod window;

fn main() {
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
    app.connect_open(|app, files, _hints| {
        for f in files {
            window::open_window(&app, f.get_uri().to_string());
        }
    });
    app.connect_activate(move |app| {
        window::open_window(&app, "about:blank".to_string());
    });
    app.run(&env::args().collect::<Vec<_>>());
}
