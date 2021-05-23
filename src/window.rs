use gio::prelude::*;
use gtk::prelude::*;
use std::convert::TryInto;

use cairo::ImageSurface;
use gdk::pixbuf_get_from_surface;
use gio::{AppInfo, AppInfoExt};
use glib::GString;
use gtk::{
    Application, ApplicationWindow, BoxBuilder, Button, FileChooserAction, FileChooserDialog,
    HeaderBarExt, IconSize, Image, Label, Menu, MenuButton, MenuItem, ModelButtonBuilder,
    Orientation, PopoverMenu, ResponseType,
};
use gtk_macros::action;
use webkit2gtk::{
    BackForwardListExt, BackForwardListItemExt, ContextMenu, ContextMenuExt, ContextMenuItem,
    DownloadExt, HitTestResultExt, WebContextExt, WebViewExt,
};

use crate::faviconheaderbar;
use crate::viewer;

pub struct Window {
    pub widget: ApplicationWindow,
    application: Application,
    header: faviconheaderbar::FaviconHeaderBar,
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

        let header = faviconheaderbar::FaviconHeaderBar::new();
        header.set_show_close_button(true);
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
            header,
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
            glib::clone!(@weak self.header as header => move |webview| {
                if let Some(title) = webview.get_title() {
                    header.set_title(Some(&title));
                } else {
                    header.set_title(None);
                }
            }),
        );

        self.viewer.webview.connect_property_uri_notify(
            glib::clone!(@weak self.header as header => move |webview| {
                if let Some(uri) = webview.get_uri() {
                    header.set_subtitle(Some(uri.as_str()));
                } else {
                    header.set_subtitle(None);
                }
            }),
        );

        self.viewer.webview.connect_property_favicon_notify(
            glib::clone!(@weak self.header as header => move |webview| {
                if let Some(surface) = webview.get_favicon() {
                    let image_surface: ImageSurface = surface.try_into().expect("image surface expected");
                    let width = image_surface.get_width();
                    let height = image_surface.get_height();
                    let pixbuf = pixbuf_get_from_surface(&image_surface, 0, 0, width, height).unwrap();

                    header.set_favicon(Some(&pixbuf));
                } else {
                    header.set_favicon(None);
                }
            }),
        );

        self.viewer.webview.get_context().unwrap().connect_download_started(glib::clone!(
                @weak self.widget as window => move |_context, download| {
            download.connect_decide_destination(move |download, suggested_filename| {
                let dialog = FileChooserDialog::with_buttons(Some("Download File"), Some(&window), FileChooserAction::Save, &[("_Cancel", ResponseType::Cancel), ("_Save", ResponseType::Accept)]);
                dialog.set_default_response(ResponseType::Accept);
                dialog.set_do_overwrite_confirmation(true);
                if let Some(download_folder) = glib::get_user_special_dir(glib::UserDirectory::Downloads) {
                    dialog.set_current_folder(&download_folder);
                }
                dialog.set_current_name(&suggested_filename);
                let res = dialog.run();
                if res == gtk::ResponseType::Accept {
                    let filename = dialog.get_uri().unwrap();
                    download.set_destination(&filename);
                } else {
                    download.cancel();
                }
                dialog.close();
                false
            });
        }));

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
