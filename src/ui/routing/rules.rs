use crate::api::routing::Routing;
use adw::prelude::*;
use relm4::{adw, gtk};
use rust_i18n::t;

pub fn view(buffer: &gtk::TextBuffer, editable: bool) -> adw::Bin {
    let text = gtk::TextView::builder()
        .buffer(buffer)
        .monospace(true)
        .editable(editable)
        .left_margin(12)
        .right_margin(12)
        .top_margin(12)
        .bottom_margin(12)
        .build();

    text.update_property(&[
        gtk::accessible::Property::Label(t!("routing.rules").as_ref()),
        gtk::accessible::Property::Required(editable),
    ]);

    let scroll = gtk::ScrolledWindow::builder()
        .vexpand(true)
        .min_content_height(200)
        .child(&text)
        .build();

    adw::Bin::builder()
        .css_classes(["card"])
        .overflow(gtk::Overflow::Hidden)
        .child(&scroll)
        .build()
}

impl Routing {
    pub fn details(&self) -> adw::Dialog {
        let dialog = adw::Dialog::builder()
            .title(&self.name)
            .content_width(740)
            .content_height(560)
            .build();

        let buffer = gtk::TextBuffer::new(None);
        buffer.set_text(&self.routing.string);

        let content = view(&buffer, false);
        content.set_margin_start(18);
        content.set_margin_end(18);
        content.set_margin_bottom(18);

        let toolbar = adw::ToolbarView::new();
        toolbar.add_top_bar(&adw::HeaderBar::new());
        toolbar.set_content(Some(&content));

        dialog.set_child(Some(&toolbar));

        dialog
    }
}
