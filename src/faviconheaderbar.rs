use gtk::gdk_pixbuf::{InterpType, Pixbuf};
use gtk::glib;
use gtk::pango::EllipsizeMode;
use gtk::prelude::*;
use gtk::subclass::prelude::*;
use once_cell::unsync::OnceCell;

use gtk::builders::{BoxBuilder, LabelBuilder};
use gtk::traits::{HeaderBarExt, LabelExt};
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
        title_box.pack_start(&favicon, true, true, 0);
        let title = LabelBuilder::new()
            .wrap(false)
            .single_line_mode(true)
            .ellipsize(EllipsizeMode::End)
            .width_chars(MIN_TITLE_CHARS)
            .halign(Align::Start)
            .build();
        title.style_context().add_class("title");
        title_box.pack_start(&title, true, true, 0);
        label_box.pack_start(&title_box, false, false, 0);

        let subtitle = LabelBuilder::new()
            .wrap(false)
            .single_line_mode(true)
            .ellipsize(EllipsizeMode::End)
            .selectable(true)
            .build();
        subtitle.style_context().add_class("subtitle");
        let subtitle_box = gtk::Box::new(Orientation::Horizontal, 0);
        subtitle_box.pack_start(&subtitle, true, false, 0);
        label_box.pack_start(&subtitle_box, false, false, 0);

        Self {
            widget: label_box,
            title,
            subtitle,
            favicon,
        }
    }
}

#[derive(Debug)]
pub struct FaviconHeaderBarPriv {
    favicontitle: OnceCell<FaviconTitle>,
}

impl Default for FaviconHeaderBarPriv {
    fn default() -> Self {
        Self {
            favicontitle: OnceCell::new(),
        }
    }
}

#[glib::object_subclass]
impl ObjectSubclass for FaviconHeaderBarPriv {
    const NAME: &'static str = "FaviconHeaderBar";
    type Type = FaviconHeaderBar;
    type ParentType = HeaderBar;
}

impl ObjectImpl for FaviconHeaderBarPriv {
    fn constructed(&self, obj: &Self::Type) {
        self.parent_constructed(obj);

        let favicontitle = FaviconTitle::new();
        obj.set_custom_title(Some(&favicontitle.widget));

        self.favicontitle
            .set(favicontitle)
            .expect("Failed to initialize private state");
    }
}

impl HeaderBarImpl for FaviconHeaderBarPriv {}
impl ContainerImpl for FaviconHeaderBarPriv {}
impl WidgetImpl for FaviconHeaderBarPriv {}

glib::wrapper! {
    pub struct FaviconHeaderBar(ObjectSubclass<FaviconHeaderBarPriv>)
        @extends gtk::HeaderBar, gtk::Container, gtk::Widget;
}

impl FaviconHeaderBar {
    pub fn new() -> Self {
        glib::Object::new(&[]).expect("Faled to create FaviconHeaderBar")
    }

    pub fn set_title(&self, title: Option<&str>) {
        let (label, tooltip) = match title {
            Some(label) => (label, Some(label)),
            None => ("", None),
        };
        let favicontitle = self.get_favicontitle();
        favicontitle.title.set_label(label);
        favicontitle.title.set_tooltip_text(tooltip);
    }

    pub fn set_subtitle(&self, title: Option<&str>) {
        let label = match title {
            Some(label) => label,
            None => "",
        };
        self.get_favicontitle().subtitle.set_label(label);
    }

    pub fn set_favicon(&self, favicon: Option<&Pixbuf>) {
        let favicontitle = self.get_favicontitle();

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

    fn get_favicontitle(&self) -> &FaviconTitle {
        let priv_ = FaviconHeaderBarPriv::from_instance(self);
        priv_
            .favicontitle
            .get()
            .expect("Private state not initialized")
    }
}
