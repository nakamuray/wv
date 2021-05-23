use gdk_pixbuf::{InterpType, Pixbuf};
use gio::prelude::*;
use glib::subclass;
use glib::translate::*;
use gtk::prelude::*;
use gtk::subclass::prelude::*;
use once_cell::unsync::OnceCell;
use pango::EllipsizeMode;

use gtk::{
    Align, BoxBuilder, HeaderBar, HeaderBarExt, Image, Label, LabelBuilder, LabelExt, Orientation,
};

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
        favicon.get_style_context().add_class("favicon");
        favicon.set_halign(Align::End);
        title_box.pack_start(&favicon, true, true, 0);
        let title = LabelBuilder::new()
            .wrap(false)
            .single_line_mode(true)
            .ellipsize(EllipsizeMode::End)
            .width_chars(MIN_TITLE_CHARS)
            .halign(Align::Start)
            .build();
        title.get_style_context().add_class("title");
        title_box.pack_start(&title, true, true, 0);
        label_box.pack_start(&title_box, false, false, 0);

        let subtitle = LabelBuilder::new()
            .wrap(false)
            .single_line_mode(true)
            .ellipsize(EllipsizeMode::End)
            .selectable(true)
            .build();
        subtitle.get_style_context().add_class("subtitle");
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

impl ObjectSubclass for FaviconHeaderBarPriv {
    const NAME: &'static str = "FaviconHeaderBar";
    type ParentType = HeaderBar;
    type Instance = subclass::simple::InstanceStruct<Self>;
    type Class = subclass::simple::ClassStruct<Self>;

    glib::glib_object_subclass!();

    fn new() -> Self {
        Self {
            favicontitle: OnceCell::new(),
        }
    }
}

impl ObjectImpl for FaviconHeaderBarPriv {
    glib::glib_object_impl!();

    fn constructed(&self, obj: &glib::Object) {
        self.parent_constructed(obj);

        let this = obj.downcast_ref::<FaviconHeaderBar>().unwrap();

        let favicontitle = FaviconTitle::new();
        this.set_custom_title(Some(&favicontitle.widget));

        self.favicontitle
            .set(favicontitle)
            .expect("Failed to initialize private state");
    }
}

impl HeaderBarImpl for FaviconHeaderBarPriv {}
impl ContainerImpl for FaviconHeaderBarPriv {}
impl WidgetImpl for FaviconHeaderBarPriv {}

glib::glib_wrapper! {
    pub struct FaviconHeaderBar(
        Object<subclass::simple::InstanceStruct<FaviconHeaderBarPriv>,
        subclass::simple::ClassStruct<FaviconHeaderBarPriv>,
        FaviconHeaderBarClass>)
        @extends gtk::HeaderBar, gtk::Container, gtk::Widget;

    match fn {
        get_type => || FaviconHeaderBarPriv::get_type().to_glib(),
    }
}

impl FaviconHeaderBar {
    pub fn new() -> Self {
        glib::Object::new(Self::static_type(), &[])
            .expect("Faled to create FaviconHeaderBar")
            .downcast()
            .expect("Created FaviconHeaderBar is of wrong type")
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
            let scale = favicontitle.favicon.get_scale_factor();
            let favicon_size = FAVICON_SIZE * scale;

            if favicon_size != favicon.get_width() || favicon_size != favicon.get_height() {
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
