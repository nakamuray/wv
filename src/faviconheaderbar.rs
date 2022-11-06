use gtk4 as gtk;

use gtk::gdk_pixbuf::{InterpType, Pixbuf};
use gtk::pango::EllipsizeMode;
use gtk::prelude::*;

use gtk::builders::{BoxBuilder, LabelBuilder};
use gtk::{Align, HeaderBar, Image, Label, Orientation};

#[derive(Debug)]
struct FaviconTitle {
    widget: gtk::Box,
    title: Label,
    subtitle: Label,
    favicon: Image,
}

const MIN_TITLE_CHARS: i32 = 6;
const FAVICON_SIZE: i32 = 16;

impl FaviconTitle {
    fn new() -> Self {
        let label_box = BoxBuilder::new()
            .orientation(Orientation::Vertical)
            .spacing(0)
            .valign(Align::Center)
            .build();

        let title_box = BoxBuilder::new()
            .orientation(Orientation::Horizontal)
            .spacing(0)
            .build();
        let favicon = Image::new();
        favicon.style_context().add_class("favicon");
        favicon.set_halign(Align::End);
        favicon.set_hexpand(true);
        title_box.append(&favicon);
        let title = LabelBuilder::new()
            .wrap(false)
            .single_line_mode(true)
            .ellipsize(EllipsizeMode::End)
            .width_chars(MIN_TITLE_CHARS)
            .halign(Align::Start)
            .build();
        title.style_context().add_class("title");
        title.set_hexpand(true);
        title_box.append(&title);
        label_box.append(&title_box);

        let subtitle = LabelBuilder::new()
            .wrap(false)
            .single_line_mode(true)
            .ellipsize(EllipsizeMode::End)
            .selectable(true)
            .build();
        subtitle.style_context().add_class("subtitle");
        let subtitle_box = gtk::Box::new(Orientation::Horizontal, 0);
        subtitle.set_hexpand(true);
        subtitle_box.append(&subtitle);
        label_box.append(&subtitle_box);

        Self {
            widget: label_box,
            title,
            subtitle,
            favicon,
        }
    }
}

#[derive(Debug)]
pub struct FaviconHeaderBar {
    pub widget: HeaderBar,
    favicontitle: FaviconTitle,
}

impl FaviconHeaderBar {
    pub fn new() -> Self {
        let favicontitle = FaviconTitle::new();
        let bar = HeaderBar::builder()
            .title_widget(&favicontitle.widget)
            .build();
        Self {
            widget: bar,
            favicontitle: favicontitle,
        }
    }

    pub fn set_title(&self, title: Option<&str>) {
        let (label, tooltip) = match title {
            Some(label) => (label, Some(label)),
            None => ("", None),
        };
        self.favicontitle.title.set_label(label);
        self.favicontitle.title.set_tooltip_text(tooltip);
    }

    pub fn set_subtitle(&self, title: Option<&str>) {
        let label = match title {
            Some(label) => label,
            None => "",
        };
        self.favicontitle.subtitle.set_label(label);
    }

    pub fn set_favicon(&self, favicon: Option<&Pixbuf>) {
        let favicontitle = &self.favicontitle;

        if let Some(favicon) = favicon {
            let scale = favicontitle.favicon.scale_factor();
            let favicon_size = FAVICON_SIZE * scale;

            if favicon_size != favicon.width() || favicon_size != favicon.height() {
                let favicon = &favicon
                    .scale_simple(favicon_size, favicon_size, InterpType::Bilinear)
                    .unwrap();
                favicontitle.favicon.set_from_pixbuf(Some(favicon));
            } else {
                favicontitle.favicon.set_from_pixbuf(Some(favicon));
            }
        } else {
            favicontitle.favicon.clear();
        }
    }

    pub fn select_subtitle(&self) {
        let subtitle = &self.favicontitle.subtitle;
        subtitle.grab_focus();
        subtitle.select_region(0, -1);
    }
}
