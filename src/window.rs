use gtk::prelude::*;
use std::cell::RefCell;
use std::convert::TryInto;
use std::rc::Rc;
use std::time::Duration;

use gtk::builders::{BoxBuilder, ModelButtonBuilder};
use gtk::cairo::ImageSurface;
use gtk::gdk::pixbuf_get_from_surface;
use gtk::gio::{traits::AppInfoExt, AppInfo};
use gtk::glib::{clone, GString};
use gtk::traits::{HeaderBarExt, SearchBarExt};
use gtk::{gio, glib};
use gtk::{
    Application, ApplicationWindow, Button, FileChooserAction, FileChooserDialog, IconSize, Image,
    Label, Menu, MenuButton, MenuItem, Orientation, PopoverMenu, ResponseType,
};
use gtk_macros::action;
use webkit2gtk::traits::{
    BackForwardListExt, BackForwardListItemExt, ContextMenuExt, DownloadExt, HitTestResultExt,
    URIRequestExt, WebContextExt, WebViewExt,
};
use webkit2gtk::{ContextMenu, ContextMenuItem, NavigationType};

use crate::faviconheaderbar;
use crate::settings::Settings;
use crate::viewer;

pub struct Window {
    pub widget: ApplicationWindow,
    application: Application,
    pub settings: Rc<RefCell<Settings>>,
    header: faviconheaderbar::FaviconHeaderBar,
    back_button: Button,
    forward_button: Button,
    reload_or_stop_button: Button,
    viewer: viewer::Viewer,
}

impl Window {
    pub fn new(app: &Application, settings: Rc<RefCell<Settings>>) -> Self {
        let win = ApplicationWindow::new(app);
        win.set_title("Web View");
        win.set_default_size(
            settings.borrow().window.width,
            settings.borrow().window.height,
        );

        let viewer = viewer::Viewer::new();
        win.add(&viewer.widget);

        let header = faviconheaderbar::FaviconHeaderBar::new();
        header.set_show_close_button(true);
        win.set_titlebar(Some(&header));

        let navigation_buttons = gtk::Box::new(Orientation::Horizontal, 0);
        navigation_buttons.style_context().add_class("linked");

        let back_button =
            Button::from_icon_name(Some("go-previous-symbolic"), IconSize::SmallToolbar);
        back_button.set_sensitive(false);
        back_button.set_tooltip_text(Some("go back"));
        navigation_buttons.pack_start(&back_button, false, false, 0);

        let forward_button =
            Button::from_icon_name(Some("go-next-symbolic"), IconSize::SmallToolbar);
        forward_button.set_sensitive(false);
        forward_button.set_tooltip_text(Some("go forward"));
        navigation_buttons.pack_start(&forward_button, false, false, 0);

        header.pack_start(&navigation_buttons);

        let reload_or_stop_button = Button::from_icon_name(
            Some("emblem-synchronizing-symbolic"),
            IconSize::SmallToolbar,
        );
        reload_or_stop_button.set_tooltip_text(Some("reload"));
        header.pack_start(&reload_or_stop_button);

        let menu_button = MenuButton::new();
        menu_button.set_image(Some(&Image::from_icon_name(
            Some("document-send-symbolic"),
            IconSize::SmallToolbar,
        )));
        menu_button.set_tooltip_text(Some("re-open page with ..."));
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

        let browsers = AppInfo::recommended_for_type("x-scheme-handler/http");
        for info in browsers.iter() {
            if info.id() == Some(GString::from("wv.desktop")) {
                // skip myself
                continue;
            };
            let button = ModelButtonBuilder::new()
                .always_show_image(true)
                .image(&Image::from_gicon(
                    &info.icon().unwrap(),
                    IconSize::SmallToolbar,
                ))
                .label(&info.name())
                .halign(gtk::Align::Start)
                .build();
            menu_box.pack_start(&button, false, false, 0);

            button.connect_clicked(
                clone!(@strong info, @weak viewer.webview as webview => move |_button| {
                    if let Some(uri) = webview.uri() {
                        if let Err(e) = info.launch_uris(&[&uri], gio::AppLaunchContext::NONE) {
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
            settings,
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
        self.widget.connect_size_allocate(
            glib::clone!(@strong self.settings as settings => move |win, _rect| {
                let (width, height) = win.size();
                (*settings.borrow_mut()).window.height = height;
                (*settings.borrow_mut()).window.width = width;
            }),
        );

        self.viewer.webview.connect_context_menu(
            |_webview, context_menu, _event, hit_test_result| {
                if hit_test_result.context_is_link() {
                    let uri = hit_test_result.link_uri().unwrap().to_string();

                    let browsers = AppInfo::recommended_for_type("x-scheme-handler/http");
                    let open_link_menu = ContextMenu::new();

                    for info in browsers.iter() {
                        if info.id() == Some(GString::from("wv.desktop")) {
                            // skip myself
                            continue;
                        };
                        let action = gio::SimpleAction::new(&info.id().unwrap(), None);
                        let name = info.name();
                        action.connect_activate(
                    glib::clone!(@strong info, @strong uri => move |_action, _parameter| {
                        if let Err(e) = info.launch_uris(&[&uri], gio::AppLaunchContext::NONE) {
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
                reload_or_stop_button.set_tooltip_text(Some("stop"));
            } else {
                reload_or_stop_button.set_image(Some(&Image::from_icon_name(Some("emblem-synchronizing-symbolic"), IconSize::SmallToolbar)));
                reload_or_stop_button.set_tooltip_text(Some("reload"));
            }
        }));

        self.viewer.webview.connect_title_notify(
            glib::clone!(@weak self.header as header => move |webview| {
                if let Some(title) = webview.title() {
                    header.set_title(Some(&title));
                } else {
                    header.set_title(None);
                }
            }),
        );

        self.viewer.webview.connect_uri_notify(
            glib::clone!(@weak self.header as header => move |webview| {
                if let Some(uri) = webview.uri() {
                    header.set_subtitle(Some(uri.as_str()));
                } else {
                    header.set_subtitle(None);
                }
            }),
        );

        self.viewer.webview.connect_favicon_notify(
            glib::clone!(@weak self.header as header => move |webview| {
                if let Some(surface) = webview.favicon() {
                    let image_surface: ImageSurface = surface.try_into().expect("image surface expected");
                    let width = image_surface.width();
                    let height = image_surface.height();
                    let pixbuf = pixbuf_get_from_surface(&image_surface, 0, 0, width, height).unwrap();

                    header.set_favicon(Some(&pixbuf));
                } else {
                    header.set_favicon(None);
                }
            }),
        );

        self.viewer.webview.context().unwrap().connect_download_started(glib::clone!(
                @weak self.widget as window => move |_context, download| {
            download.connect_decide_destination(move |download, suggested_filename| {
                let dialog = FileChooserDialog::with_buttons(Some("Download File"), Some(&window), FileChooserAction::Save, &[("_Cancel", ResponseType::Cancel), ("_Save", ResponseType::Accept)]);
                dialog.set_default_response(ResponseType::Accept);
                dialog.set_do_overwrite_confirmation(true);
                if let Some(download_folder) = glib::user_special_dir(glib::UserDirectory::Downloads) {
                    dialog.set_current_folder(&download_folder);
                }
                dialog.set_current_name(&suggested_filename);
                let res = dialog.run();
                if res == gtk::ResponseType::Accept {
                    let filename = dialog.uri().unwrap();
                    download.set_destination(&filename);
                } else {
                    download.cancel();
                }
                dialog.close();
                false
            });
        }));

        self.viewer.webview.connect_create(glib::clone!(
                @weak self.application as app,
                @strong self.settings as settings => @default-return None, move |_webview, navigation_action| {
            if navigation_action.navigation_type() == NavigationType::Other {
                if let Some(req) = navigation_action.request() {
                    if let Some(uri) = req.uri() {
                        // action from "Open Link in New Window" context menu (maybe)
                        let win = Window::new(&app, settings.clone());
                        win.widget.show_all();
                        win.load_uri(&uri);
                    }
                }
            }
            None
        }));

        // XXX: until BackForwardListExt::connect_changed is implemented,
        // poll to check we can go back/forward
        glib::timeout_add_local(
            Duration::from_secs(1),
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
            match (event.button(), webview.back_forward_list()) {
                (3, Some(back_forward_list)) => {
                    let menu = Menu::new();
                    for back in back_forward_list.back_list() {
                        let item = MenuItem::new();
                        item.set_label(&back.title().unwrap_or(GString::from("(no title)")));
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
            match (event.button(), webview.back_forward_list()) {
                (3, Some(back_forward_list)) => {
                    let menu = Menu::new();
                    // put list items in reverse order
                    for forward in back_forward_list.forward_list().iter().rev() {
                        let item = MenuItem::new();
                        item.set_label(&forward.title().unwrap_or(GString::from("(no title)")));
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
                if !search_bar.is_search_mode() {
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

        action!(
            self.widget,
            "select-url",
            glib::clone!(@weak self.header as header => move |_action, _parameter| {
                header.select_subtitle();
            })
        );
        self.application
            .set_accels_for_action("win.select-url", &["<Primary>l"]);
    }
    pub fn load_uri(&self, uri: &str) {
        self.viewer.webview.load_uri(uri)
    }
}
