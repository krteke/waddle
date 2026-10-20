use adw::prelude::*;
use gtk::glib;
use relm4::{RelmWidgetExt, adw, gtk};
use rust_i18n::t;

use crate::{app::AppMsg, settings::Server};

const PAGES: [(&str, &str, &str); 5] = [
    ("overview", "navigation.overview", "view-grid-symbolic"),
    ("groups", "navigation.groups", "view-list-symbolic"),
    ("nodes", "navigation.nodes", "network-server-symbolic"),
    (
        "subscriptions",
        "navigation.subscriptions",
        "application-rss+xml-symbolic",
    ),
    (
        "configurations",
        "navigation.configurations",
        "document-properties-symbolic",
    ),
];

pub struct Workspace {
    pub view: adw::OverlaySplitView,
    pub connection_content: gtk::Box,
    pub overview: gtk::Stack,
    server_name: gtk::Label,
    server: gtk::MenuButton,
    server_menu: gtk::Popover,
    language: gtk::MenuButton,
    servers: gtk::Button,
    toggle: gtk::ToggleButton,
    hide: gtk::Button,
    error: adw::Banner,
    pages: Vec<(gtk::Label, adw::StatusPage)>,
}

impl Workspace {
    pub fn new(language_menu: &gtk::gio::Menu) -> Self {
        let view = adw::OverlaySplitView::builder()
            .min_sidebar_width(220.0)
            .max_sidebar_width(280.0)
            .sidebar_width_fraction(0.3)
            .build();

        let server_name = gtk::Label::builder()
            .xalign(0.0)
            .hexpand(true)
            .ellipsize(gtk::pango::EllipsizeMode::End)
            .build();

        server_name.add_css_class("heading");

        let server_content = gtk::Box::new(gtk::Orientation::Horizontal, 12);
        server_content.append(&server_name);
        server_content.append(&gtk::Image::from_icon_name("pan-down-symbolic"));

        let server = gtk::MenuButton::builder().child(&server_content).build();

        server.set_create_popup_func(|button| {
            if button.popover().is_none() {
                gtk::prelude::WidgetExt::activate_action(
                    button,
                    "navigation.push",
                    Some(&"servers".to_variant()),
                )
                .unwrap();
                button.popdown();
            }
        });

        server.add_css_class("flat");

        let connection_content = gtk::Box::new(gtk::Orientation::Vertical, 0);
        connection_content.set_margin_start(12);
        connection_content.set_margin_end(12);

        let stack = gtk::Stack::builder()
            .hexpand(true)
            .vexpand(true)
            .transition_type(gtk::StackTransitionType::Crossfade)
            .build();

        let navigation = gtk::ListBox::builder()
            .selection_mode(gtk::SelectionMode::Single)
            .build();

        navigation.add_css_class("navigation-sidebar");

        let overview = gtk::Stack::new();
        let mut pages = Vec::new();

        PAGES.into_iter().for_each(|(name, _, icon)| {
            let label = gtk::Label::builder().xalign(0.0).build();
            let content = gtk::Box::new(gtk::Orientation::Horizontal, 12);

            content.set_margin_all(10);
            content.append(&gtk::Image::from_icon_name(icon));
            content.append(&label);

            let row = gtk::ListBoxRow::builder().child(&content).build();
            navigation.append(&row);

            let page = adw::StatusPage::builder().icon_name(icon).build();

            if name == "overview" {
                overview.add_named(&page, Some("placeholder"));
                stack.add_named(&overview, Some(name));
            } else {
                stack.add_named(&page, Some(name));
            }

            pages.push((label, page));
        });

        navigation.connect_row_selected(gtk::glib::clone!(
            #[weak]
            stack,
            move |_, row| {
                if let Some(row) = row {
                    stack.set_visible_child_name(PAGES[row.index() as usize].0);
                }
            }
        ));

        navigation.connect_row_activated(gtk::glib::clone!(
            #[weak]
            view,
            move |list, row| {
                list.select_row(Some(row));
                if view.is_collapsed() {
                    view.set_show_sidebar(false);
                }
            }
        ));
        navigation.select_row(navigation.row_at_index(0).as_ref());

        let sidebar_body = gtk::Box::new(gtk::Orientation::Vertical, 12);
        sidebar_body.set_margin_all(12);
        sidebar_body.append(&server);
        sidebar_body.append(&connection_content);
        sidebar_body.append(&gtk::Separator::new(gtk::Orientation::Horizontal));
        sidebar_body.append(&navigation);

        let scroll = gtk::ScrolledWindow::builder()
            .hscrollbar_policy(gtk::PolicyType::Never)
            .child(&sidebar_body)
            .vexpand(true)
            .build();

        let hide = gtk::Button::from_icon_name("pan-start-symbolic");

        view.bind_property("collapsed", &hide, "visible")
            .sync_create()
            .build();
        hide.connect_clicked(gtk::glib::clone!(
            #[weak]
            view,
            move |_| view.set_show_sidebar(false)
        ));

        let sidebar_header = adw::HeaderBar::builder()
            .show_end_title_buttons(false)
            .title_widget(&gtk::Label::new(Some("Waddle")))
            .build();
        sidebar_header.pack_end(&hide);

        let sidebar = adw::ToolbarView::new();
        sidebar.add_top_bar(&sidebar_header);
        sidebar.set_content(Some(&scroll));

        view.set_sidebar(Some(&sidebar));

        let toggle = gtk::ToggleButton::builder()
            .icon_name("sidebar-show-symbolic")
            .build();

        view.bind_property("show-sidebar", &toggle, "active")
            .bidirectional()
            .sync_create()
            .build();

        let language = gtk::MenuButton::builder()
            .icon_name("preferences-desktop-locale-symbolic")
            .menu_model(language_menu)
            .build();

        let servers = gtk::Button::builder()
            .icon_name("network-server-symbolic")
            .action_name("navigation.push")
            .action_target(&"servers".to_variant())
            .build();

        let header = adw::HeaderBar::builder()
            .show_start_title_buttons(false)
            .title_widget(&gtk::Label::new(None))
            .build();

        header.pack_start(&toggle);
        header.pack_end(&language);
        header.pack_end(&servers);

        let error = adw::Banner::new("");
        let content = adw::ToolbarView::new();

        content.add_top_bar(&header);
        content.add_top_bar(&error);
        content.set_content(Some(&stack));

        view.set_content(Some(&content));

        Self {
            view,
            connection_content,
            overview,
            server_name,
            server,
            server_menu: gtk::Popover::new(),
            language,
            servers,
            toggle,
            hide,
            error,
            pages,
        }
    }

    pub fn refresh(
        &self,
        records: &[Server],
        active: Option<usize>,
        error: Option<&str>,
        sender: &relm4::Sender<AppMsg>,
    ) {
        self.server_name.set_label(
            active
                .map(|index| records[index].name.as_str())
                .unwrap_or(t!("app.servers").as_ref()),
        );

        self.server
            .set_tooltip_text(Some(t!("server.select").as_ref()));

        if records.is_empty() {
            self.server.set_popover(None::<&gtk::Popover>);
        } else {
            let list = gtk::Box::new(gtk::Orientation::Vertical, 4);
            let popover = &self.server_menu;

            for (index, record) in records.iter().enumerate() {
                let content = gtk::Box::new(gtk::Orientation::Horizontal, 12);

                let name = gtk::Label::builder()
                    .label(&record.name)
                    .xalign(0.0)
                    .hexpand(true)
                    .ellipsize(gtk::pango::EllipsizeMode::End)
                    .max_width_chars(24)
                    .build();

                content.append(&name);

                if active == Some(index) {
                    content.append(&gtk::Image::from_icon_name("object-select-symbolic"));
                }

                let button = gtk::Button::builder()
                    .child(&content)
                    .sensitive(record.ip.is_ok())
                    .build();

                button.add_css_class("flat");

                if record.ip.is_err() {
                    button.set_tooltip_text(Some(t!("server.invalid_ip").as_ref()));
                }

                button.connect_clicked(glib::clone!(
                    #[weak]
                    popover,
                    #[strong]
                    sender,
                    move |_| {
                        popover.popdown();
                        sender.emit(AppMsg::OpenServer(index));
                    }
                ));

                list.append(&button);
            }

            let scroll = gtk::ScrolledWindow::builder()
                .hscrollbar_policy(gtk::PolicyType::Never)
                .min_content_width(180)
                .propagate_natural_width(true)
                .max_content_height(360)
                .propagate_natural_height(true)
                .child(&list)
                .build();

            popover.set_child(Some(&scroll));

            if list.measure(gtk::Orientation::Vertical, -1).1 <= scroll.max_content_height() {
                scroll.set_vscrollbar_policy(gtk::PolicyType::Never);
            }

            if self.server.popover().is_none() {
                self.server.set_popover(Some(popover));
            }
        }

        self.language
            .set_tooltip_text(Some(t!("language.title").as_ref()));

        self.language
            .update_property(&[gtk::accessible::Property::Label(
                t!("language.title").as_ref(),
            )]);

        self.servers
            .set_tooltip_text(Some(t!("app.servers").as_ref()));
        self.servers
            .update_property(&[gtk::accessible::Property::Label(t!("app.servers").as_ref())]);

        self.toggle
            .set_tooltip_text(Some(t!("navigation.toggle_sidebar").as_ref()));
        self.toggle
            .update_property(&[gtk::accessible::Property::Label(
                t!("navigation.toggle_sidebar").as_ref(),
            )]);

        self.hide
            .set_tooltip_text(Some(t!("navigation.hide_sidebar").as_ref()));
        self.hide
            .update_property(&[gtk::accessible::Property::Label(
                t!("navigation.hide_sidebar").as_ref(),
            )]);

        self.error.set_title(error.unwrap_or_default());
        self.error.set_revealed(error.is_some());

        self.pages
            .iter()
            .zip(PAGES)
            .for_each(|((label, page), (_, key, _))| {
                label.set_label(t!(key).as_ref());
                page.set_title(t!(key).as_ref());
            });
    }
}
