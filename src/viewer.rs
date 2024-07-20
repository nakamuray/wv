use gtk4 as gtk;

use gtk::prelude::*;

use gtk::glib;
use gtk::glib::clone;

use gtk::{Align, Label, Orientation, Overlay, ProgressBar, SearchBar, SearchEntry};
use webkit6::prelude::*;
use webkit6::{
    CookieAcceptPolicy, FindOptions, NavigationPolicyDecision, NetworkSession, PolicyDecisionType,
    WebView,
};

pub struct Viewer {
    pub widget: gtk::Box,
    pub webview: WebView,
    progress_bar: ProgressBar,
    status_bar: Label,
    pub search_bar: SearchBar,
    search_entry: SearchEntry,
    match_count_label: Label,
}

impl Viewer {
    pub fn new() -> Self {
        let box_ = gtk::Box::new(Orientation::Vertical, 0);
        box_.set_homogeneous(false);
        let overlay = Overlay::new();
        overlay.set_vexpand(true);
        box_.prepend(&overlay);

        let network_session = NetworkSession::new_ephemeral();
        network_session
            .cookie_manager()
            .unwrap()
            .set_accept_policy(CookieAcceptPolicy::NoThirdParty);
        network_session.set_itp_enabled(true);
        if let Some(website_data_manager) = network_session.website_data_manager() {
            website_data_manager.set_favicons_enabled(true);
        }
        let webview = glib::object::Object::builder()
            .property("network-session", network_session)
            .build();
        WebViewExt::settings(&webview)
            .unwrap()
            .set_enable_smooth_scrolling(true);
        overlay.set_child(Some(&webview));

        let progress_bar = ProgressBar::builder()
            .halign(gtk::Align::Fill)
            .valign(gtk::Align::Start)
            .can_target(false)
            .fraction(0.0)
            .build();
        progress_bar.set_visible(false);
        overlay.add_overlay(&progress_bar);

        let status_bar = Label::builder()
            .halign(gtk::Align::Start)
            .valign(gtk::Align::End)
            .can_target(false)
            .build();
        status_bar.add_css_class("status-bar");
        status_bar.set_visible(false);
        overlay.add_overlay(&status_bar);

        let search_bar = SearchBar::new();
        search_bar.set_show_close_button(true);
        box_.append(&search_bar);

        let search_box = gtk::Box::new(Orientation::Horizontal, 6);
        search_bar.set_child(Some(&search_box));

        let search_entry = gtk::SearchEntry::new();
        search_entry.set_halign(Align::Start);
        search_bar.connect_entry(&search_entry);
        search_entry.set_hexpand(true);
        search_box.prepend(&search_entry);

        let match_count_label = Label::new(None);
        match_count_label.set_halign(Align::End);
        search_box.append(&match_count_label);

        let this = Self {
            widget: box_,
            webview,
            progress_bar,
            status_bar,
            search_bar,
            search_entry,
            match_count_label,
        };
        this.setup_callbacks();
        this
    }
    fn setup_callbacks(&self) {
        self.webview.connect_estimated_load_progress_notify(clone!(
            #[weak(rename_to = progress_bar)]
            self.progress_bar,
            move |webview| {
                if webview.is_loading() {
                    progress_bar.set_visible(true);
                    progress_bar.set_fraction(webview.estimated_load_progress());
                } else {
                    progress_bar.set_visible(false);
                }
            }
        ));
        self.webview.connect_load_changed(glib::clone!(
            #[weak(rename_to = progress_bar)]
            self.progress_bar,
            move |_webview, event| {
                if event == webkit6::LoadEvent::Finished {
                    progress_bar.set_visible(false);
                }
            }
        ));

        self.webview.connect_mouse_target_changed(glib::clone!(
            #[weak(rename_to = status_bar)]
            self.status_bar,
            move |_webview, hit_test_result, _modofiers| {
                if hit_test_result.context_is_link() {
                    status_bar.set_label(&hit_test_result.link_uri().unwrap());
                    status_bar.set_visible(true);
                } else {
                    status_bar.set_visible(false);
                }
            }
        ));

        self.webview
            .connect_decide_policy(|webview, decision, decision_type| {
                match decision_type {
                    PolicyDecisionType::NewWindowAction => {
                        let navigation_decision: &NavigationPolicyDecision =
                            decision.downcast_ref().unwrap();
                        let mut action = navigation_decision.navigation_action().unwrap();
                        let request = action.request().unwrap();
                        // open link in this window, not new window
                        webview.load_request(&request);
                        true
                    }
                    _ => false,
                }
            });

        let find_controller = self.webview.find_controller().unwrap();
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
                            (FindOptions::WRAP_AROUND & FindOptions::CASE_INSENSITIVE).bits(),
                            std::u32::MAX,
                        );
                        find_controller.search(
                            &search_text,
                            (FindOptions::WRAP_AROUND & FindOptions::CASE_INSENSITIVE).bits(),
                            std::u32::MAX,
                        );
                    }
                }
            }
        ));
        self.search_entry.connect_stop_search(glib::clone!(
            #[weak]
            find_controller,
            #[weak(rename_to = match_count_label)]
            self.match_count_label,
            move |_search_entry| {
                match_count_label.set_label("");
                find_controller.search_finish();
            }
        ));
        find_controller.connect_counted_matches(glib::clone!(
            #[weak(rename_to = match_count_label)]
            self.match_count_label,
            move |_find_controller, match_count| {
                match_count_label.set_label(&format!("{} matches", match_count));
            }
        ));
    }
}
