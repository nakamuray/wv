use gtk4 as gtk;

use gtk::prelude::*;
use std::cell::RefCell;
use std::rc::Rc;

use gtk::gio::AppInfo;
use gtk::glib::{clone, GString};
use gtk::{gdk, gio, glib};
use gtk::{
    gio::{File, SimpleAction},
    Align, Application, ApplicationWindow, Button, HeaderBar, Image, Label, MenuButton,
    Orientation, Popover,
};
use webkit6::prelude::*;
use webkit6::{
    ContextMenu, ContextMenuItem, NavigationPolicyDecision, NavigationType, PolicyDecisionType,
    WebView,
};

use crate::favicontitle;
use crate::settings::Settings;
use crate::viewer;

pub struct Window {
    pub widget: ApplicationWindow,
    application: Application,
    pub settings: Rc<RefCell<Settings>>,
    favicontitle: favicontitle::FaviconTitle,
    back_button: Button,
    forward_button: Button,
    reload_or_stop_button: Button,
    viewer: viewer::Viewer,
}

impl Window {
    pub fn new(
        app: &Application,
        settings: Rc<RefCell<Settings>>,
        related_view: Option<&WebView>,
    ) -> Self {
        let win = ApplicationWindow::new(app);
        win.set_title(Some("Web View"));
        win.set_default_size(
            settings.borrow().window.width,
            settings.borrow().window.height,
        );

        let viewer = viewer::Viewer::new(related_view);
        win.set_child(Some(&viewer.widget));

        let favicontitle = favicontitle::FaviconTitle::new();
        let header = HeaderBar::builder().title_widget(&favicontitle).build();
        header.set_show_title_buttons(true);
        win.set_titlebar(Some(&header));

        let navigation_buttons = gtk::Box::new(Orientation::Horizontal, 0);
        navigation_buttons.add_css_class("linked");

        let back_button = Button::from_icon_name("go-previous-symbolic");
        back_button.set_sensitive(false);
        back_button.set_tooltip_text(Some("go back"));
        navigation_buttons.append(&back_button);

        let forward_button = Button::from_icon_name("go-next-symbolic");
        forward_button.set_sensitive(false);
        forward_button.set_tooltip_text(Some("go forward"));
        navigation_buttons.append(&forward_button);

        header.pack_start(&navigation_buttons);

        let reload_or_stop_button = Button::from_icon_name("view-refresh-symbolic");
        reload_or_stop_button.set_tooltip_text(Some("reload"));
        header.pack_start(&reload_or_stop_button);

        let menu_button = MenuButton::new();
        menu_button.set_icon_name("document-send-symbolic");
        menu_button.set_tooltip_text(Some("re-open page with ..."));
        header.pack_end(&menu_button);

        let menu_popover = Popover::new();
        menu_button.set_popover(Some(&menu_popover));

        let menu_box = gtk::Box::builder()
            .orientation(Orientation::Vertical)
            .spacing(0)
            .margin_top(10)
            .margin_bottom(10)
            .margin_start(10)
            .margin_end(10)
            .build();
        menu_popover.set_child(Some(&menu_box));

        let label = Label::new(Some("Re-Open Page with ..."));
        menu_box.append(&label);

        let browsers = AppInfo::recommended_for_type("x-scheme-handler/http");
        for info in browsers.iter() {
            if info.id() == Some(GString::from("wv.desktop")) {
                // skip myself
                continue;
            };
            let hbox = gtk::Box::new(Orientation::Horizontal, 4);
            if let Some(icon) = info.icon() {
                hbox.prepend(&Image::from_gicon(&icon));
            }
            hbox.append(&Label::new(Some(&info.name())));
            let button = Button::builder()
                .has_frame(false)
                .child(&hbox)
                .halign(Align::Start)
                .hexpand(true)
                .build();
            button.add_css_class("menuitem");
            menu_box.append(&button);

            button.connect_clicked(clone!(
                #[strong]
                info,
                #[weak(rename_to = webview)]
                viewer.webview,
                #[weak]
                menu_popover,
                move |_button| {
                    if let Some(uri) = webview.uri() {
                        if let Err(e) = info.launch_uris(&[&uri], gio::AppLaunchContext::NONE) {
                            eprintln!("{:?}", e);
                        }
                    }
                    menu_popover.popdown();
                }
            ));
        }

        let this = Self {
            widget: win,
            application: app.clone(),
            settings,
            favicontitle,
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
        self.widget.connect_default_height_notify(glib::clone!(
            #[strong(rename_to = settings)]
            self.settings,
            move |win| {
                let height = win.size(Orientation::Vertical);
                let width = win.size(Orientation::Horizontal);
                (*settings.borrow_mut()).window.height = height;
                (*settings.borrow_mut()).window.width = width;
            }
        ));
        self.widget.connect_default_width_notify(glib::clone!(
            #[strong(rename_to = settings)]
            self.settings,
            move |win| {
                let height = win.size(Orientation::Vertical);
                let width = win.size(Orientation::Horizontal);
                (*settings.borrow_mut()).window.height = height;
                (*settings.borrow_mut()).window.width = width;
            }
        ));

        self.viewer
            .webview
            .connect_context_menu(|_webview, context_menu, hit_test_result| {
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
                        action.connect_activate(glib::clone!(
                            #[strong]
                            info,
                            #[strong]
                            uri,
                            move |_action, _parameter| {
                                if let Err(e) =
                                    info.launch_uris(&[&uri], gio::AppLaunchContext::NONE)
                                {
                                    eprintln!("{:?}", e);
                                }
                            }
                        ));
                        let item = webkit6::ContextMenuItem::from_gaction(&action, &name, None);
                        open_link_menu.append(&item);
                    }
                    let open_link_item =
                        ContextMenuItem::with_submenu("Open Link with ...", &open_link_menu);
                    context_menu.insert(&open_link_item, 2);
                }
                false
            });

        self.viewer.webview.connect_load_changed(glib::clone!(
            #[weak(rename_to = back_button)]
            self.back_button,
            #[weak(rename_to = forward_button)]
            self.forward_button,
            #[weak(rename_to = reload_or_stop_button)]
            self.reload_or_stop_button,
            move |webview, _event| {
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
                    reload_or_stop_button.set_icon_name("process-stop-symbolic");
                    reload_or_stop_button.set_tooltip_text(Some("stop"));
                } else {
                    reload_or_stop_button.set_icon_name("view-refresh-symbolic");
                    reload_or_stop_button.set_tooltip_text(Some("reload"));
                }
            }
        ));

        self.viewer.webview.connect_title_notify(glib::clone!(
            #[weak(rename_to = favicontitle)]
            self.favicontitle,
            move |webview| {
                if let Some(title) = webview.title() {
                    favicontitle.set_title(Some(&title));
                } else {
                    favicontitle.set_title(None);
                }
            }
        ));

        self.viewer.webview.connect_uri_notify(glib::clone!(
            #[weak(rename_to = favicontitle)]
            self.favicontitle,
            move |webview| {
                if let Some(uri) = webview.uri() {
                    favicontitle.set_subtitle(Some(uri.as_str()));
                } else {
                    favicontitle.set_subtitle(None);
                }
            }
        ));

        self.viewer.webview.connect_favicon_notify(glib::clone!(
            #[weak(rename_to = favicontitle)]
            self.favicontitle,
            move |webview| {
                if let Some(favicon) = webview.favicon() {
                    favicontitle.set_favicon(Some(&favicon));
                } else {
                    favicontitle.set_favicon(None);
                }
            }
        ));

        self.viewer
            .webview
            .network_session()
            .unwrap()
            .connect_download_started(glib::clone!(
                #[weak(rename_to = window)]
                self.widget,
                move |_context, download| {
                    download.connect_decide_destination(move |download, suggested_filename| {
                        let suggested_filename = suggested_filename.to_owned();
                        let window = window.clone();
                        let download = download.clone();
                        glib::MainContext::default().spawn_local(async move {
                            let dialog = gtk::FileDialog::builder()
                                .title("Download File")
                                .initial_name(&suggested_filename)
                                .build();
                            if let Some(download_folder) =
                                glib::user_special_dir(glib::UserDirectory::Downloads)
                            {
                                dialog.set_initial_folder(Some(&File::for_path(&download_folder)));
                            }
                            if let Ok(file) = dialog.save_future(Some(&window)).await {
                                if let Some(path) = file.path() {
                                    download.set_destination(&path.to_string_lossy());
                                } else {
                                    eprintln!("path is None for {}", file.uri());
                                    download.cancel();
                                }
                            } else {
                                download.cancel();
                            }
                        });

                        true
                    });
                }
            ));

        self.viewer.webview.connect_create(glib::clone!(
            #[weak(rename_to = app)]
            self.application,
            #[strong(rename_to = settings)]
            self.settings,
            #[upgrade_or]
            glib::object::Object::builder().build(),
            move |webview, navigation_action| {
                // XXX: why navigation_action.navigation_type() require &mut?
                let mut navigation_action = navigation_action.clone();
                if navigation_action.navigation_type() == NavigationType::Other {
                    if let Some(req) = navigation_action.request() {
                        if let Some(_uri) = req.uri() {
                            // action from "Open Link in New Window" context menu (maybe)
                            let win = Window::new(&app, settings.clone(), Some(&webview));
                            win.viewer.webview.connect_ready_to_show(glib::clone!(
                                #[weak(rename_to = window)]
                                win.widget,
                                move |_webview| {
                                    window.present();
                                }
                            ));
                            return win.viewer.webview.into();
                        }
                    }
                }
                webview.clone().into()
            }
        ));
        self.viewer.webview.connect_decide_policy(glib::clone!(
            #[weak(rename_to = app)]
            self.application,
            #[strong(rename_to = settings)]
            self.settings,
            #[upgrade_or]
            false,
            move |webview, decision, decision_type| {
                match decision_type {
                    PolicyDecisionType::NavigationAction | PolicyDecisionType::NewWindowAction => {
                        ()
                    }
                    _ => return false,
                }

                let navigation_decision: &NavigationPolicyDecision =
                    decision.downcast_ref().unwrap();
                let mut action = navigation_decision.navigation_action().unwrap();

                let button = action.mouse_button();
                let state = action.modifiers();
                if (action.is_user_gesture()
                    || action.navigation_type() == NavigationType::LinkClicked)
                    && (button == gdk::BUTTON_MIDDLE
                        || state == gdk::ModifierType::SHIFT_MASK.bits()
                        || state == gdk::ModifierType::CONTROL_MASK.bits())
                {
                    let request = action.request().unwrap();
                    if let Some(uri) = request.uri() {
                        // open link in new window
                        let win = Window::new(&app, settings.clone(), Some(&webview));
                        win.widget.present();
                        win.load_uri(&uri);
                        decision.ignore();
                        return true;
                    }
                }

                if decision_type == PolicyDecisionType::NewWindowAction {
                    if action.is_user_gesture()
                        || action.navigation_type() == NavigationType::LinkClicked
                    {
                        let request = action.request().unwrap();
                        // open link in this window, not new window
                        webview.load_request(&request);
                        decision.ignore();
                        return true;
                    } else {
                        // ignore automatically opened new window
                        decision.ignore();
                        return true;
                    }
                }
                false
            }
        ));

        self.viewer
            .webview
            .back_forward_list()
            .unwrap()
            .connect_local(
                "changed",
                false,
                glib::clone!(
                    #[weak(rename_to = webview)]
                    self.viewer.webview,
                    #[weak(rename_to = back_button)]
                    self.back_button,
                    #[weak(rename_to = forward_button)]
                    self.forward_button,
                    #[upgrade_or]
                    None,
                    move |_args| {
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
                        None
                    }
                ),
            );

        self.back_button.connect_clicked(glib::clone!(
            #[weak(rename_to = webview)]
            self.viewer.webview,
            move |_button| {
                webview.go_back();
                webview.grab_focus();
            }
        ));
        let back_button_right_pressed = gtk::GestureClick::new();
        // right mouse button
        back_button_right_pressed.set_button(3);
        back_button_right_pressed.connect_pressed(glib::clone!(
            #[weak(rename_to = webview)]
            self.viewer.webview,
            #[weak(rename_to = back_button)]
            self.back_button,
            move |_gesture, _n, _x, _y| {
                if let Some(popover) = build_history_popover(&webview, HistoryDirection::Back) {
                    popover.set_parent(&back_button);
                    popover.popup();
                }
            }
        ));
        self.back_button.add_controller(back_button_right_pressed);

        self.forward_button.connect_clicked(glib::clone!(
            #[weak(rename_to = webview)]
            self.viewer.webview,
            move |_button| {
                webview.go_forward();
                webview.grab_focus();
            }
        ));
        let forward_button_right_pressed = gtk::GestureClick::new();
        // right mouse button
        forward_button_right_pressed.set_button(3);
        forward_button_right_pressed.connect_pressed(glib::clone!(
            #[weak(rename_to = webview)]
            self.viewer.webview,
            #[weak(rename_to = forward_button)]
            self.forward_button,
            move |_gesture, _n, _x, _y| {
                if let Some(popover) = build_history_popover(&webview, HistoryDirection::Forward) {
                    popover.set_parent(&forward_button);
                    popover.popup();
                }
            }
        ));
        self.forward_button
            .add_controller(forward_button_right_pressed);
        self.reload_or_stop_button.connect_clicked(glib::clone!(
            #[weak(rename_to = webview)]
            self.viewer.webview,
            move |_button| {
                if webview.is_loading() {
                    webview.stop_loading();
                } else {
                    webview.reload();
                }
                webview.grab_focus();
            }
        ));
    }
    fn setup_accels(&self) {
        let close_action = SimpleAction::new("close", None);
        close_action.connect_activate(glib::clone!(
            #[weak(rename_to = window)]
            self.widget,
            move |_action, _parameter| {
                window.close();
            }
        ));
        self.widget.add_action(&close_action);
        self.application
            .set_accels_for_action("win.close", &["<Primary>w"]);

        let find_action = SimpleAction::new("find", None);
        find_action.connect_activate(glib::clone!(
            #[weak(rename_to = search_bar)]
            self.viewer.search_bar,
            move |_action, _parameter| {
                if !search_bar.is_search_mode() {
                    search_bar.set_search_mode(true);
                }
            }
        ));
        self.widget.add_action(&find_action);
        self.application
            .set_accels_for_action("win.find", &["<Primary>f"]);

        let back_action = SimpleAction::new("back", None);
        back_action.connect_activate(glib::clone!(
            #[weak(rename_to = webview)]
            self.viewer.webview,
            move |_action, _parameter| {
                webview.go_back();
            }
        ));
        self.widget.add_action(&back_action);
        self.application
            .set_accels_for_action("win.back", &["<alt>Left"]);

        let forward_action = SimpleAction::new("forward", None);
        forward_action.connect_activate(glib::clone!(
            #[weak(rename_to = webview)]
            self.viewer.webview,
            move |_action, _parameter| {
                webview.go_forward();
            }
        ));
        self.widget.add_action(&forward_action);
        self.application
            .set_accels_for_action("win.forward", &["<alt>Right"]);

        let reload_action = SimpleAction::new("reload", None);
        reload_action.connect_activate(glib::clone!(
            #[weak(rename_to = webview)]
            self.viewer.webview,
            move |_action, _parameter| {
                webview.reload();
            }
        ));
        self.widget.add_action(&reload_action);
        self.application
            .set_accels_for_action("win.reload", &["<Primary>r"]);

        let selecturl_action = SimpleAction::new("select-url", None);
        selecturl_action.connect_activate(glib::clone!(
            #[weak(rename_to = favicontitle)]
            self.favicontitle,
            move |_action, _parameter| {
                favicontitle.select_subtitle();
            }
        ));
        self.widget.add_action(&selecturl_action);
        self.application
            .set_accels_for_action("win.select-url", &["<Primary>l"]);
    }
    pub fn load_uri(&self, uri: &str) {
        self.viewer.webview.load_uri(uri)
    }
}

enum HistoryDirection {
    Back,
    Forward,
}

fn build_history_popover(webview: &WebView, direction: HistoryDirection) -> Option<Popover> {
    let back_forward_list = webview.back_forward_list()?;

    let popover = Popover::new();
    let menu_box = gtk::Box::builder()
        .orientation(Orientation::Vertical)
        .spacing(0)
        .margin_top(10)
        .margin_bottom(10)
        .margin_start(10)
        .margin_end(10)
        .build();
    popover.set_child(Some(&menu_box));
    popover.connect_closed(|menu| {
        // to destroy this menu, unparent it
        menu.unparent();
    });

    let list = match direction {
        HistoryDirection::Back => back_forward_list.back_list(),
        HistoryDirection::Forward => back_forward_list.forward_list(),
    };
    for item in list {
        let label = Label::builder()
            .ellipsize(gtk::pango::EllipsizeMode::End)
            .max_width_chars(48)
            .single_line_mode(true)
            .xalign(0.0)
            .hexpand(true)
            .build();
        if let Some(title) = &item.title() {
            label.set_text(title);
        } else {
            label.set_text(&item.uri().unwrap_or(GString::from("(no title)")));
        }
        let button = Button::builder()
            .has_frame(false)
            .halign(Align::Fill)
            .hexpand(true)
            .build();
        button.set_child(Some(&label));
        button.add_css_class("menuitem");
        button.connect_clicked(glib::clone!(
            #[weak]
            webview,
            #[weak]
            popover,
            move |_button| {
                popover.popdown();
                webview.go_to_back_forward_list_item(&item);
                webview.grab_focus();
            }
        ));
        menu_box.append(&button);
    }

    Some(popover)
}
