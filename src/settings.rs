use gtk4 as gtk;

use gtk::gio::prelude::*;
use gtk::gio::{Cancellable, File};
use gtk::{gio, glib};
use serde_derive::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct Settings {
    pub window: Window,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Window {
    #[serde(default = "default_width")]
    pub width: i32,

    #[serde(default = "default_height")]
    pub height: i32,
}

fn default_width() -> i32 {
    800
}
fn default_height() -> i32 {
    640
}

fn get_app_config_dir() -> Option<std::path::PathBuf> {
    let mut path = glib::user_config_dir();
    path.push("wv");
    Some(path)
}

const SETTINGS_FILE_NAME: &'static str = "settings.toml";

pub fn load_settings() -> Settings {
    if let Some(mut settings_path) = get_app_config_dir() {
        let settings_dir = File::for_path(&settings_path);
        if !settings_dir.query_exists(Cancellable::NONE) {
            settings_dir
                .make_directory_with_parents(Cancellable::NONE)
                .unwrap_or_else(|e| {
                    // TODO: log
                    dbg!(e);
                });
        }
        settings_path.push(SETTINGS_FILE_NAME);
        let settings_file = File::for_path(settings_path);
        if let Ok((data, _)) = settings_file.load_contents(Cancellable::NONE) {
            match std::str::from_utf8(&data) {
                Ok(s) => match toml::from_str(&s) {
                    Ok(settings) => {
                        return settings;
                    }
                    Err(e) => {
                        dbg!(&e);
                    }
                },
                Err(e) => {
                    dbg!(&e);
                }
            }
        }
    }
    Settings {
        window: Window {
            height: default_height(),
            width: default_width(),
        },
    }
}

pub fn save_settings(settings: &Settings) {
    if let Some(mut settings_path) = get_app_config_dir() {
        let settings_dir = gio::File::for_path(&settings_path);
        if !settings_dir.query_exists(Cancellable::NONE) {
            settings_dir
                .make_directory_with_parents(Cancellable::NONE)
                .unwrap_or_else(|e| {
                    // TODO: log
                    dbg!(e);
                });
        }
        settings_path.push(SETTINGS_FILE_NAME);
        let settings_data = toml::to_string(settings).unwrap();
        let settings_file = gio::File::for_path(settings_path);
        if let Err(e) = settings_file.replace_contents(
            settings_data.as_bytes(),
            None,
            false,
            gio::FileCreateFlags::NONE,
            Cancellable::NONE,
        ) {
            dbg!(e);
        }
    }
}
