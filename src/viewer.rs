use gtk4 as gtk;

use gtk::glib;
use gtk::subclass::prelude::*;

use webkit6::prelude::*;
use webkit6::{CookieAcceptPolicy, NetworkSession, WebView};

mod imp {
    use gtk::glib;
    use gtk::glib::clone;
    use gtk::subclass::prelude::*;
    use gtk::{Align, Label, Orientation, Overlay, ProgressBar, SearchBar, SearchEntry};
    use gtk4 as gtk;
    use std::cell::OnceCell;
    use webkit6::prelude::*;
    use webkit6::{FindOptions, WebView};

    #[derive(glib::Properties, Debug)]
    #[properties(wrapper_type = super::Viewer)]
    pub struct Viewer {
        box_: gtk::Box,
        #[property(get)]
        pub webview: OnceCell<WebView>,
        pub(super) overlay: Overlay,
        progress_bar: ProgressBar,
        status_bar: Label,
        #[property(get)]
        pub search_bar: SearchBar,
        search_entry: SearchEntry,
        match_count_label: Label,
        alert_revealer: gtk::Revealer,
        alert_label: Label,
    }
    impl Default for Viewer {
        fn default() -> Self {
            let box_ = gtk::Box::new(Orientation::Vertical, 0);
            box_.set_homogeneous(false);

            let overlay = Overlay::new();
            overlay.set_vexpand(true);

            let progress_bar = ProgressBar::builder()
                .halign(gtk::Align::Fill)
                .valign(gtk::Align::Start)
                .can_target(false)
                .fraction(0.0)
                .build();
            progress_bar.set_visible(false);

            let status_bar = Label::builder()
                .halign(gtk::Align::Start)
                .valign(gtk::Align::End)
                .can_target(false)
                .build();
            status_bar.add_css_class("status-bar");
            status_bar.set_visible(false);

            let alert_label = Label::builder()
                .css_classes(["alert"])
                .hexpand(true)
                .justify(gtk::Justification::Center)
                .build();
            let alert_revealer = gtk::Revealer::builder()
                .hexpand(true)
                .child(&alert_label)
                .build();

            let search_bar = SearchBar::new();
            search_bar.set_show_close_button(true);

            let search_entry = gtk::SearchEntry::new();
            search_entry.set_halign(Align::Start);
            search_entry.set_hexpand(true);

            let match_count_label = Label::new(None);
            match_count_label.set_halign(Align::End);

            Self {
                box_,
                webview: OnceCell::new(),
                overlay,
                progress_bar,
                status_bar,
                search_bar,
                search_entry,
                match_count_label,
                alert_revealer,
                alert_label,
            }
        }
    }
    #[glib::object_subclass]
    impl ObjectSubclass for Viewer {
        const NAME: &'static str = "Viewer";
        type Type = super::Viewer;
        type ParentType = gtk::Widget;
    }
    #[glib::derived_properties]
    impl ObjectImpl for Viewer {
        fn constructed(&self) {
            self.parent_constructed();

            self.box_.set_parent(&*self.obj());

            self.box_.prepend(&self.overlay);

            self.overlay.add_overlay(&self.progress_bar);
            self.overlay.add_overlay(&self.status_bar);

            self.box_.prepend(&self.alert_revealer);
            self.box_.append(&self.search_bar);

            let search_box = gtk::Box::new(Orientation::Horizontal, 6);
            self.search_bar.set_child(Some(&search_box));
            self.search_bar.connect_entry(&self.search_entry);
            search_box.prepend(&self.search_entry);
            search_box.append(&self.match_count_label);

            //self.setup_callbacks();
        }
        fn dispose(&self) {
            self.box_.unparent();
        }
    }
    impl WidgetImpl for Viewer {
        fn measure(&self, orientation: gtk::Orientation, for_size: i32) -> (i32, i32, i32, i32) {
            self.box_.measure(orientation, for_size)
        }
        fn size_allocate(&self, width: i32, height: i32, baseline: i32) {
            self.box_
                .size_allocate(&gtk::Allocation::new(0, 0, width, height), baseline)
        }
    }

    impl Viewer {
        pub(super) fn setup_callbacks(&self) {
            let webview = self
                .webview
                .get()
                .expect("setup_callbacks should be called after initializing webview");
            webview.connect_estimated_load_progress_notify(clone!(
                #[weak(rename_to = this)]
                self,
                move |webview| {
                    if webview.is_loading() {
                        this.progress_bar.set_visible(true);
                        this.progress_bar
                            .set_fraction(webview.estimated_load_progress());
                    } else {
                        this.progress_bar.set_visible(false);
                    }
                }
            ));
            webview.connect_load_changed(glib::clone!(
                #[weak(rename_to = this)]
                self,
                move |_webview, event| {
                    if event == webkit6::LoadEvent::Finished {
                        this.progress_bar.set_visible(false);
                    }
                }
            ));

            webview.connect_mouse_target_changed(glib::clone!(
                #[weak(rename_to = this)]
                self,
                move |_webview, hit_test_result, _modofiers| {
                    if hit_test_result.context_is_link() {
                        this.status_bar
                            .set_label(&hit_test_result.link_uri().unwrap());
                        this.status_bar.set_visible(true);
                    } else {
                        this.status_bar.set_visible(false);
                    }
                }
            ));

            let find_controller = webview.find_controller().unwrap();
            self.search_entry.connect_activate(glib::clone!(
                #[weak]
                find_controller,
                move |search_entry| {
                    let search_text = search_entry.text();
                    match find_controller.search_text() {
                        Some(s) if s == search_text => {
                            find_controller.search_next();
                        }
                        _ => {
                            find_controller.count_matches(
                                &search_text,
                                (FindOptions::WRAP_AROUND | FindOptions::CASE_INSENSITIVE).bits(),
                                std::u32::MAX,
                            );
                            find_controller.search(
                                &search_text,
                                (FindOptions::WRAP_AROUND | FindOptions::CASE_INSENSITIVE).bits(),
                                std::u32::MAX,
                            );
                        }
                    }
                }
            ));
            self.search_entry.connect_search_changed(glib::clone!(
                #[weak]
                find_controller,
                #[weak(rename_to = this)]
                self,
                move |search_entry| {
                    let search_text = search_entry.text();
                    if search_text.is_empty() {
                        this.match_count_label.set_label("");
                        find_controller.search_finish();
                    } else {
                        find_controller.count_matches(
                            &search_text,
                            (FindOptions::WRAP_AROUND | FindOptions::CASE_INSENSITIVE).bits(),
                            std::u32::MAX,
                        );
                        find_controller.search(
                            &search_text,
                            (FindOptions::WRAP_AROUND | FindOptions::CASE_INSENSITIVE).bits(),
                            std::u32::MAX,
                        );
                    }
                }
            ));
            self.search_entry.connect_stop_search(glib::clone!(
                #[weak]
                find_controller,
                #[weak(rename_to = this)]
                self,
                move |_search_entry| {
                    this.match_count_label.set_label("");
                    find_controller.search_finish();
                }
            ));
            find_controller.connect_counted_matches(glib::clone!(
                #[weak(rename_to = this)]
                self,
                move |_find_controller, match_count| {
                    this.match_count_label
                        .set_label(&format!("{} matches", match_count));
                }
            ));

            webview.connect_web_process_terminated(glib::clone!(
                #[weak(rename_to = this)]
                self,
                move |_webview, reason| {
                    this.alert_label
                        .set_label(&format!("web process terminated: {:?}", reason));
                    this.alert_revealer.set_reveal_child(true);
                }
            ));
            webview.connect_load_changed(glib::clone!(
                #[weak(rename_to = this)]
                self,
                move |_webview, _event| {
                    this.alert_revealer.set_reveal_child(false);
                }
            ));
        }
    }
}

glib::wrapper! {
    pub struct Viewer(ObjectSubclass<imp::Viewer>)
        @extends gtk::Widget, gtk::Box,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget, gtk::Orientable;
}

impl Viewer {
    pub fn new(related_view: Option<&WebView>) -> Self {
        let obj: Self = glib::Object::builder().build();
        let imp = obj.imp();

        let mut builder = WebView::builder();
        if let Some(related_view) = related_view {
            builder = builder.related_view(related_view);
        } else {
            let network_session = NetworkSession::new_ephemeral();
            network_session
                .cookie_manager()
                .unwrap()
                .set_accept_policy(CookieAcceptPolicy::NoThirdParty);
            network_session.set_itp_enabled(true);
            if let Some(website_data_manager) = network_session.website_data_manager() {
                website_data_manager.set_favicons_enabled(true);
            }

            builder = builder.network_session(&network_session);
        }
        let webview = builder.build();
        let settings = WebViewExt::settings(&webview).unwrap();
        settings.set_enable_smooth_scrolling(true);
        settings.set_enable_back_forward_navigation_gestures(true);

        imp.overlay.set_child(Some(&webview));
        imp.webview
            .set(webview)
            .expect("newly created object should not have webview");
        imp.setup_callbacks();

        obj
    }
}
