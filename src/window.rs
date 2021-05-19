use gio::prelude::*;
use gtk::prelude::*;
use std::convert::TryInto;

use cairo::ImageSurface;
use gdk::pixbuf_get_from_surface;
use gdk_pixbuf::InterpType;
use gio::{AppInfo, AppInfoExt};
use glib::GString;
use gtk::{
    Align, Application, ApplicationWindow, BoxBuilder, Button, HeaderBar, HeaderBarExt, IconSize,
    Image, Label, LabelBuilder, LabelExt, Menu, MenuButton, MenuItem, ModelButtonBuilder,
    Orientation, PopoverMenu,
};
use gtk_macros::action;
use pango::EllipsizeMode;
use webkit2gtk::{
    BackForwardListExt, BackForwardListItemExt, ContextMenu, ContextMenuExt, ContextMenuItem,
    HitTestResultExt, WebViewExt,
};

use crate::viewer;

struct CustomTitle {
    widget: gtk::Box,
    title: Label,
    subtitle: Label,
    favicon: Image,
}

const MIN_TITLE_CHARS: i32 = 6;

impl CustomTitle {
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
        label_box.pack_start(&subtitle, false, false, 0);

        Self {
            widget: label_box,
            title,
            subtitle,
            favicon,
        }
    }
}

pub struct Window {
    pub widget: ApplicationWindow,
    application: Application,
    title: CustomTitle,
    back_button: Button,
    forward_button: Button,
    reload_or_stop_button: Button,
    viewer: viewer::Viewer,
}

impl Window {
    pub fn new(app: &Application) -> Self {
        let win = ApplicationWindow::new(app);
        win.set_title("Web View");

        let viewer = viewer::Viewer::new();
        win.add(&viewer.widget);

        let header = HeaderBar::new();
        header.set_show_close_button(true);
        let title = CustomTitle::new();
        header.set_custom_title(Some(&title.widget));
        win.set_titlebar(Some(&header));

        let navigation_buttons = gtk::Box::new(Orientation::Horizontal, 0);
        navigation_buttons.get_style_context().add_class("linked");

        let back_button =
            Button::from_icon_name(Some("go-previous-symbolic"), IconSize::SmallToolbar);
        back_button.set_sensitive(false);
        navigation_buttons.pack_start(&back_button, false, false, 0);

        let forward_button =
            Button::from_icon_name(Some("go-next-symbolic"), IconSize::SmallToolbar);
        forward_button.set_sensitive(false);
        navigation_buttons.pack_start(&forward_button, false, false, 0);

        header.pack_start(&navigation_buttons);

        let reload_or_stop_button = Button::from_icon_name(
            Some("emblem-synchronizing-symbolic"),
            IconSize::SmallToolbar,
        );
        header.pack_start(&reload_or_stop_button);

        let menu_button = MenuButton::new();
        menu_button.set_image(Some(&Image::from_icon_name(
            Some("document-send-symbolic"),
            IconSize::SmallToolbar,
        )));
        header.pack_end(&menu_button);

        let menu_popover = PopoverMenu::new();
        menu_button.set_popover(Some(&menu_popover));

        let menu_box = BoxBuilder::new()
            .orientation(Orientation::Vertical)
            .spacing(0)
            .margin_top(10)
            .margin_bottom(10)
            .margin_start(10)
            .margin_end(10)
            .build();
        menu_popover.add(&menu_box);

        let label = Label::new(Some("Re-Open Page with ..."));
        menu_box.pack_start(&label, false, false, 0);

        let browsers = AppInfo::get_recommended_for_type("x-scheme-handler/http");
        for info in browsers.iter() {
            if info.get_id() == Some(GString::from("wv.desktop")) {
                // skip myself
                continue;
            };
            let button = ModelButtonBuilder::new()
                .always_show_image(true)
                .image(&Image::from_gicon(
                    &info.get_icon().unwrap(),
                    IconSize::SmallToolbar,
                ))
                .label(&info.get_name().unwrap())
                .halign(gtk::Align::Start)
                .build();
            menu_box.pack_start(&button, false, false, 0);

            button.connect_clicked(
                glib::clone!(@strong info, @weak viewer.webview as webview => move |_button| {
                    if let Some(uri) = webview.get_uri() {
                        if let Err(e) = info.launch_uris::<gio::AppLaunchContext>(&[&uri], None) {
                            eprintln!("{:?}", e);
                        }
                    }
                }),
            );
        }

        menu_box.show_all();

        let this = Self {
            widget: win,
            application: app.clone(),
            title,
            back_button,
            forward_button,
            reload_or_stop_button,
            viewer,
        };
        this.connect_signals();
        this.setup_accels();
        this
    }
    fn connect_signals(&self) {
        self.viewer.webview.connect_context_menu(
            |_webview, context_menu, _event, hit_test_result| {
                if hit_test_result.context_is_link() {
                    let uri = hit_test_result.get_link_uri().unwrap().to_string();

                    let browsers = AppInfo::get_recommended_for_type("x-scheme-handler/http");
                    let open_link_menu = ContextMenu::new();

                    for info in browsers.iter() {
                        if info.get_id() == Some(GString::from("wv.desktop")) {
                            // skip myself
                            continue;
                        };
                        let action = gio::SimpleAction::new(&info.get_id().unwrap(), None);
                        let name = info.get_name().unwrap();
                        action.connect_activate(
                    glib::clone!(@strong info, @strong uri => move |_action, _parameter| {
                        if let Err(e) = info.launch_uris::<gio::AppLaunchContext>(&[&uri], None) {
                            eprintln!("{:?}", e);
                        }
                    }),
                );
                        let item = webkit2gtk::ContextMenuItem::from_gaction(&action, &name, None);
                        open_link_menu.append(&item);
                    }
                    let open_link_item =
                        ContextMenuItem::with_submenu("Open Link with ...", &open_link_menu);
                    context_menu.insert(&open_link_item, 2);
                }
                false
            },
        );

        self.viewer.webview.connect_load_changed(glib::clone!(
                @weak self.back_button as back_button,
                @weak self.forward_button as forward_button,
                @weak self.reload_or_stop_button as reload_or_stop_button => move |webview, _event| {
            if webview.can_go_back() {
                back_button.set_sensitive(true);
            } else {
                back_button.set_sensitive(false);
            }
            if webview.can_go_forward() {
                forward_button.set_sensitive(true);
            } else {
                forward_button.set_sensitive(false);
            }

            if webview.is_loading() {
                reload_or_stop_button.set_image(Some(&Image::from_icon_name(Some("process-stop-symbolic"), IconSize::SmallToolbar)));
            } else {
                reload_or_stop_button.set_image(Some(&Image::from_icon_name(Some("emblem-synchronizing-symbolic"), IconSize::SmallToolbar)));
            }
        }));

        self.viewer.webview.connect_property_title_notify(
            glib::clone!(@weak self.title.title as title_label => move |webview| {
                if let Some(title) = webview.get_title() {
                    title_label.set_label(title.as_str());
                    title_label.set_tooltip_text(Some(title.as_str()));
                } else {
                    title_label.set_label("");
                    title_label.set_tooltip_text(None);
                }
            }),
        );

        self.viewer.webview.connect_property_uri_notify(
            glib::clone!(@weak self.title.subtitle as subtitle => move |webview| {
                if let Some(uri) = webview.get_uri() {
                    subtitle.set_label(uri.as_str());
                } else {
                    subtitle.set_label("");
                }
            }),
        );

        self.viewer.webview.connect_property_favicon_notify(
            glib::clone!(@weak self.title.favicon as favicon => move |webview| {
                if let Some(surface) = webview.get_favicon() {
                    let image_surface: ImageSurface = surface.try_into().expect("image surface expected");
                    let width = image_surface.get_width();
                    let height = image_surface.get_height();
                    let mut pixbuf = pixbuf_get_from_surface(&image_surface, 0, 0, width, height).unwrap();

                    const FAVICON_SIZE: i32 = 16;
                    let scale = favicon.get_scale_factor();
                    let favicon_size = FAVICON_SIZE * scale;
                    if favicon_size != width || favicon_size != height {
                        pixbuf = pixbuf.scale_simple(favicon_size, favicon_size, InterpType::Bilinear).unwrap();
                    }
                    favicon.set_from_pixbuf(Some(&pixbuf));

                } else {
                    favicon.clear();
                }
            }),
        );

        // XXX: until BackForwardListExt::connect_changed is implemented,
        // poll to check we can go back/forward
        glib::timeout_add_local(
            1000,
            glib::clone!(
                    @weak self.viewer.webview as webview,
                    @weak self.back_button as back_button,
                    @weak self.forward_button as forward_button => @default-return glib::Continue(false), move || {
                if webview.can_go_back() {
                    back_button.set_sensitive(true);
                } else {
                    back_button.set_sensitive(false);
                }
                if webview.can_go_forward() {
                    forward_button.set_sensitive(true);
                } else {
                    forward_button.set_sensitive(false);
                }
                glib::Continue(true)
            }),
        );

        self.back_button.connect_clicked(
            glib::clone!(@weak self.viewer.webview as webview => move |_button| {
                webview.go_back();
                webview.grab_focus();
            }),
        );
        self.back_button.connect_button_press_event(glib::clone!(
                @weak self.viewer.webview as webview => @default-return Inhibit(false), move |_back_button, event| {
            match (event.get_button(), webview.get_back_forward_list()) {
                (3, Some(back_forward_list)) => {
                    let menu = Menu::new();
                    for back in back_forward_list.get_back_list() {
                        let item = MenuItem::new();
                        item.set_label(&back.get_title().unwrap_or(GString::from("(no title)")));
                        item.connect_activate(glib::clone!(@weak webview => move |_item| {
                            webview.go_to_back_forward_list_item(&back);
                            webview.grab_focus();
                        }));
                        menu.add(&item);
                    }
                    menu.show_all();
                    menu.popup_at_pointer(Some(&event));
                    Inhibit(true)
                },
                _ => Inhibit(false)
            }
        }));
        self.forward_button.connect_clicked(
            glib::clone!(@weak self.viewer.webview as webview => move |_button| {
                webview.go_forward();
                webview.grab_focus();
            }),
        );
        self.forward_button.connect_button_press_event(glib::clone!(
                @weak self.viewer.webview as webview => @default-return Inhibit(false), move |_forward_button, event| {
            match (event.get_button(), webview.get_back_forward_list()) {
                (3, Some(back_forward_list)) => {
                    let menu = Menu::new();
                    // put list items in reverse order
                    for forward in back_forward_list.get_forward_list().iter().rev() {
                        let item = MenuItem::new();
                        item.set_label(&forward.get_title().unwrap_or(GString::from("(no title)")));
                        item.connect_activate(glib::clone!(@weak webview, @weak forward => move |_item| {
                            webview.go_to_back_forward_list_item(&forward);
                            webview.grab_focus();
                        }));
                        menu.add(&item);
                    }
                    menu.show_all();
                    menu.popup_at_pointer(Some(&event));
                    Inhibit(true)
                },
                _ => Inhibit(false)
            }
        }));
        self.reload_or_stop_button.connect_clicked(
            glib::clone!(@weak self.viewer.webview as webview => move |_button| {
                if webview.is_loading() {
                    webview.stop_loading();
                } else {
                    webview.reload();
                }
                webview.grab_focus();
            }),
        );
    }
    fn setup_accels(&self) {
        action!(
            self.widget,
            "close",
            glib::clone!(@weak self.widget as window => move |_action, _parameter| {
                window.close();
            })
        );
        self.application
            .set_accels_for_action("win.close", &["<Primary>w"]);

        action!(
            self.widget,
            "find",
            glib::clone!(@weak self.viewer.search_bar as search_bar => move |_action, _parameter| {
                if !search_bar.get_search_mode() {
                    search_bar.set_search_mode(true);
                }
            })
        );
        self.application
            .set_accels_for_action("win.find", &["<Primary>f"]);

        action!(
            self.widget,
            "back",
            glib::clone!(@weak self.viewer.webview as webview => move |_action, _parameter| {
                webview.go_back();
            })
        );
        self.application
            .set_accels_for_action("win.back", &["<alt>Left"]);

        action!(
            self.widget,
            "forward",
            glib::clone!(@weak self.viewer.webview as webview => move |_action, _parameter| {
                webview.go_forward();
            })
        );
        self.application
            .set_accels_for_action("win.forward", &["<alt>Right"]);

        action!(
            self.widget,
            "reload",
            glib::clone!(@weak self.viewer.webview as webview => move |_action, _parameter| {
                webview.reload();
            })
        );
        self.application
            .set_accels_for_action("win.reload", &["<Primary>r"]);
    }
    pub fn load_uri(&self, uri: &str) {
        self.viewer.webview.load_uri(uri)
    }
}
