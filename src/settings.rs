use gio::prelude::*;
use gio::{Cancellable, File};
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
    let mut path = glib::get_user_config_dir()?;
    path.push("wv");
    Some(path)
}

const SETTINGS_FILE_NAME: &'static str = "settings.toml";

pub fn load_settings() -> Settings {
    if let Some(mut settings_path) = get_app_config_dir() {
        let settings_dir = File::new_for_path(&settings_path);
        if !settings_dir.query_exists::<Cancellable>(None) {
            settings_dir
                .make_directory_with_parents::<Cancellable>(None)
                .unwrap_or_else(|e| {
                    // TODO: log
                    dbg!(e);
                });
        }
        settings_path.push(SETTINGS_FILE_NAME);
        let settings_file = File::new_for_path(settings_path);
        if let Ok((data, _)) = settings_file.load_contents::<Cancellable>(None) {
            match toml::from_slice(&data) {
                Ok(settings) => {
                    return settings;
                }
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
        let settings_dir = gio::File::new_for_path(&settings_path);
        if !settings_dir.query_exists::<gio::Cancellable>(None) {
            settings_dir
                .make_directory_with_parents::<Cancellable>(None)
                .unwrap_or_else(|e| {
                    // TODO: log
                    dbg!(e);
                });
        }
        settings_path.push(SETTINGS_FILE_NAME);
        let settings_data = toml::to_vec(settings).unwrap();
        let settings_file = gio::File::new_for_path(settings_path);
        if let Err(e) = settings_file.replace_contents::<gio::Cancellable>(
            &settings_data,
            None,
            false,
            gio::FileCreateFlags::NONE,
            None,
        ) {
            dbg!(e);
        }
    }
}
