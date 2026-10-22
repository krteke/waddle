use adw::prelude::*;
use relm4::{RelmWidgetExt, adw, gtk};
use rust_i18n::t;

use crate::{api::node::Node, ui::qr_code::QrCode};

impl Node {
    pub fn dialog(&self) -> adw::Dialog {
        let dialog = adw::Dialog::builder()
            .title(t!("nodes.share_title").as_ref())
            .content_width(420)
            .content_height(520)
            .build();

        let title = adw::WindowTitle::new(
            self.tag
                .as_deref()
                .filter(|tag| !tag.is_empty())
                .unwrap_or(&self.name),
            t!("nodes.share_title").as_ref(),
        );

        let toolbar = adw::ToolbarView::new();
        toolbar.add_top_bar(&adw::HeaderBar::builder().title_widget(&title).build());

        let body = gtk::Box::new(gtk::Orientation::Vertical, 18);

        body.set_margin_all(24);
        body.set_margin_top(6);

        match QrCode::new(&self.link, t!("nodes.qr_code").as_ref()) {
            Ok(qr) => body.append(&qr.widget),
            Err(error) => {
                tracing::warn!(%error, "Could not encode node link as a QR code");

                body.append(
                    &gtk::Label::builder()
                        .label(t!("nodes.qr_error").as_ref())
                        .wrap(true)
                        .css_classes(["error"])
                        .build(),
                );
            }
        }

        let link = gtk::TextView::builder()
            .editable(false)
            .wrap_mode(gtk::WrapMode::WordChar)
            .left_margin(12)
            .right_margin(12)
            .top_margin(12)
            .bottom_margin(12)
            .css_classes(["monospace", "inline"])
            .build();

        link.buffer().set_text(&self.link);
        link.update_property(&[gtk::accessible::Property::Label(t!("nodes.link").as_ref())]);

        body.append(
            &gtk::ScrolledWindow::builder()
                .hscrollbar_policy(gtk::PolicyType::Never)
                .min_content_height(128)
                .max_content_height(192)
                .vexpand(true)
                .propagate_natural_height(true)
                .css_classes(["card"])
                .overflow(gtk::Overflow::Hidden)
                .child(&link)
                .build(),
        );

        let copy = gtk::Button::with_label(t!("nodes.copy_link").as_ref());
        copy.set_height_request(44);

        let original_link = self.link.clone();

        copy.connect_clicked(move |button| {
            button.clipboard().set_text(&original_link);
            button.set_label(t!("nodes.copied").as_ref());
        });

        body.append(&copy);

        toolbar.set_content(Some(
            &gtk::ScrolledWindow::builder()
                .hscrollbar_policy(gtk::PolicyType::Never)
                .propagate_natural_height(true)
                .max_content_height(520)
                .child(&body)
                .build(),
        ));

        dialog.set_child(Some(&toolbar));

        dialog
    }
}
