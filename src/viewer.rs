use gtk::prelude::*;

use gtk::glib;
use gtk::glib::clone;

use gtk::builders::{LabelBuilder, ProgressBarBuilder};
use gtk::{
    traits::SearchBarExt, Align, Label, Orientation, Overlay, ProgressBar, SearchBar, SearchEntry,
};
use webkit2gtk::traits::{
    CookieManagerExt, FindControllerExt, HitTestResultExt, NavigationPolicyDecisionExt,
    SettingsExt, WebContextExt, WebViewExt,
};
use webkit2gtk::{
    CookieAcceptPolicy, FindOptions, NavigationPolicyDecision, PolicyDecisionType, WebContext,
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
        let overlay = Overlay::new();
        box_.pack_start(&overlay, true, true, 0);
        let context = WebContext::new_ephemeral();
        context.set_sandbox_enabled(true);
        context
            .cookie_manager()
            .unwrap()
            .set_accept_policy(CookieAcceptPolicy::NoThirdParty);
        context.set_favicon_database_directory(None);
        let webview = WebView::with_context(&context);
        WebViewExt::settings(&webview)
            .unwrap()
            .set_enable_smooth_scrolling(true);
        overlay.add(&webview);

        let progress_bar = ProgressBarBuilder::new()
            .halign(gtk::Align::Fill)
            .valign(gtk::Align::Start)
            .no_show_all(true)
            .fraction(0.0)
            .build();
        overlay.add_overlay(&progress_bar);
        overlay.set_overlay_pass_through(&progress_bar, true);

        let status_bar = LabelBuilder::new()
            .halign(gtk::Align::Start)
            .valign(gtk::Align::End)
            .no_show_all(true)
            .build();
        status_bar.style_context().add_class("status-bar");
        overlay.add_overlay(&status_bar);
        overlay.set_overlay_pass_through(&status_bar, true);

        let search_bar = SearchBar::new();
        search_bar.set_show_close_button(true);
        box_.pack_end(&search_bar, false, false, 0);

        let search_box = gtk::Box::new(Orientation::Horizontal, 6);
        search_bar.add(&search_box);

        let search_entry = gtk::SearchEntry::new();
        search_entry.set_halign(Align::Start);
        search_bar.connect_entry(&search_entry);
        search_box.pack_start(&search_entry, true, true, 0);

        let match_count_label = Label::new(None);
        match_count_label.set_halign(Align::End);
        search_box.pack_end(&match_count_label, false, false, 0);

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
        @weak self.progress_bar as progress_bar => move |webview| {
            if webview.is_loading() {
                progress_bar.show();
                progress_bar.set_fraction(webview.estimated_load_progress());
            } else {
                progress_bar.hide();
            }
        }));
        self.webview.connect_load_changed(glib::clone!(
            @weak self.progress_bar as progress_bar => move |_webview, event| {
                if event == webkit2gtk::LoadEvent::Finished {
                    progress_bar.hide();
                }
        }));

        self.webview.connect_mouse_target_changed(
            glib::clone!(@weak self.status_bar as status_bar => move |_webview, hit_test_result, _modofiers| {
                if hit_test_result.context_is_link() {
                    status_bar.set_label(&hit_test_result.link_uri().unwrap());
                    status_bar.show();
                } else {
                    status_bar.hide();
                }
            }),
        );

        self.webview
            .connect_decide_policy(|webview, decision, decision_type| {
                match decision_type {
                    PolicyDecisionType::NewWindowAction => {
                        let navigation_decision: &NavigationPolicyDecision =
                            decision.downcast_ref().unwrap();
                        let action = navigation_decision.navigation_action().unwrap();
                        let request = action.request().unwrap();
                        // open link in this window, not new window
                        webview.load_request(&request);
                        true
                    }
                    _ => false,
                }
            });

        let find_controller = self.webview.find_controller().unwrap();
        self.search_entry.connect_activate(glib::clone!(@weak find_controller => move |search_entry| {
            let search_text = search_entry.text();
            match find_controller.search_text() {
                Some(s) if s == search_text => {
                    find_controller.search_next();
                },
                _ => {
                    find_controller.count_matches(&search_text, (FindOptions::WRAP_AROUND & FindOptions::CASE_INSENSITIVE).bits(), std::u32::MAX);
                    find_controller.search(&search_text, (FindOptions::WRAP_AROUND & FindOptions::CASE_INSENSITIVE).bits(), std::u32::MAX);
                }
            }
        }));
        self.search_entry.connect_stop_search(glib::clone!(@weak find_controller, @weak self.match_count_label as match_count_label => move |_search_entry| {
            match_count_label.set_label("");
            find_controller.search_finish();
        }));
        find_controller.connect_counted_matches(glib::clone!(@weak self.match_count_label as match_count_label => move |_find_controller, match_count| {
            match_count_label.set_label(&format!("{} matches", match_count));
        }));
    }
}
