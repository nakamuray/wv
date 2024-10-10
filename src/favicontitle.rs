use gtk4 as gtk;

use gtk::glib;
use gtk::prelude::*;

mod imp {
    use gtk::glib;
    use gtk::pango::EllipsizeMode;
    use gtk::prelude::*;
    use gtk::subclass::prelude::*;
    use gtk::{Align, Image, Label, Orientation};
    use gtk4 as gtk;

    const MIN_TITLE_CHARS: i32 = 6;

    #[derive(glib::Properties, Default, Debug)]
    #[properties(wrapper_type = super::FaviconTitle)]
    pub struct FaviconTitle {
        #[property(get)]
        pub title: Label,
        #[property(get)]
        pub subtitle: Label,
        #[property(get)]
        pub favicon: Image,
    }
    #[glib::object_subclass]
    impl ObjectSubclass for FaviconTitle {
        const NAME: &'static str = "FaviconTitle";
        type Type = super::FaviconTitle;
        type ParentType = gtk::Box;
    }

    #[glib::derived_properties]
    impl ObjectImpl for FaviconTitle {
        fn constructed(&self) {
            self.parent_constructed();

            let label_box = self.obj();
            label_box.set_orientation(Orientation::Vertical);
            label_box.set_spacing(0);
            label_box.set_valign(Align::Center);

            let title_box = gtk::Box::builder()
                .orientation(Orientation::Horizontal)
                .spacing(0)
                .build();
            self.favicon.add_css_class("favicon");
            self.favicon.set_halign(Align::End);
            self.favicon.set_hexpand(true);
            title_box.append(&self.favicon);

            self.title.set_wrap(false);
            self.title.set_single_line_mode(true);
            self.title.set_ellipsize(EllipsizeMode::End);
            self.title.set_width_chars(MIN_TITLE_CHARS);
            self.title.set_halign(Align::Start);
            self.title.add_css_class("title");
            self.title.set_hexpand(true);
            title_box.append(&self.title);
            label_box.append(&title_box);

            self.subtitle.set_wrap(false);
            self.subtitle.set_single_line_mode(true);
            self.subtitle.set_ellipsize(EllipsizeMode::End);
            self.subtitle.set_selectable(true);
            self.subtitle.add_css_class("subtitle");
            let subtitle_box = gtk::Box::new(Orientation::Horizontal, 0);
            self.subtitle.set_hexpand(true);
            subtitle_box.append(&self.subtitle);
            label_box.append(&subtitle_box);
        }
    }
    impl WidgetImpl for FaviconTitle {}
    impl BoxImpl for FaviconTitle {}
}

glib::wrapper! {
    pub struct FaviconTitle(ObjectSubclass<imp::FaviconTitle>)
        @extends gtk::Widget, gtk::Box,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget, gtk::Orientable;
}

impl FaviconTitle {
    pub fn new() -> Self {
        glib::Object::builder().build()
    }

    pub fn set_title(&self, title: Option<&str>) {
        let (label, tooltip) = match title {
            Some(label) => (label, Some(label)),
            None => ("", None),
        };
        self.title().set_label(label);
        self.title().set_tooltip_text(tooltip);
    }

    pub fn set_subtitle(&self, title: Option<&str>) {
        let label = match title {
            Some(label) => label,
            None => "",
        };
        self.subtitle().set_label(label);
    }

    pub fn set_favicon(&self, favicon: Option<&gtk::gdk::Texture>) {
        if let Some(favicon) = favicon {
            self.favicon().set_paintable(Some(favicon));
        } else {
            self.favicon().clear();
        }
    }

    pub fn select_subtitle(&self) {
        let subtitle = &self.subtitle();
        subtitle.grab_focus();
        subtitle.select_region(0, -1);
    }
}
