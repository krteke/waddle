use adw::prelude::*;
use relm4::{Sender, adw, gtk};
use rust_i18n::t;

use super::NodeMsg;
use crate::{
    api::node::Node,
    ui::latency::{Latency, LatencyIndicator},
};

pub struct NodeRow {
    pub node: Node,
    pub widget: adw::ActionRow,
    pub latency: Latency,
    indicator: LatencyIndicator,
    edit: gtk::Button,
    share: gtk::Button,
    delete: gtk::Button,
}

impl NodeRow {
    pub fn new(node: Node, input: Sender<NodeMsg>) -> Self {
        let widget = adw::ActionRow::builder()
            .title(
                node.tag
                    .as_deref()
                    .filter(|tag| !tag.is_empty())
                    .unwrap_or(&node.name),
            )
            .subtitle(format!("{} · {}", node.protocol.to_uppercase(), node.name))
            .use_markup(false)
            .title_lines(1)
            .subtitle_lines(1)
            .build();

        widget.set_tooltip_text(Some(&node.name));
        widget.add_prefix(
            &gtk::Image::builder()
                .icon_name("network-workgroup-symbolic")
                .icon_size(gtk::IconSize::Large)
                .css_classes(["dim-label"])
                .build(),
        );

        let suffix = gtk::Box::builder()
            .orientation(gtk::Orientation::Vertical)
            .valign(gtk::Align::Center)
            .spacing(4)
            .margin_top(8)
            .margin_bottom(8)
            .build();

        let actions = gtk::Box::new(gtk::Orientation::Horizontal, 0);

        let edit = gtk::Button::builder()
            .icon_name("document-edit-symbolic")
            .css_classes(["flat"])
            .build();

        let share = gtk::Button::builder()
            .icon_name("insert-link-symbolic")
            .css_classes(["flat"])
            .build();

        let delete = gtk::Button::builder()
            .icon_name("user-trash-symbolic")
            .css_classes(["flat", "error"])
            .build();

        let id = node.id.clone();
        let edit_input = input.clone();
        edit.connect_clicked(move |_| edit_input.emit(NodeMsg::Edit(id.clone())));

        let id = node.id.clone();
        let share_input = input.clone();
        share.connect_clicked(move |_| share_input.emit(NodeMsg::Share(id.clone())));

        let id = node.id.clone();
        delete.connect_clicked(move |_| input.emit(NodeMsg::ConfirmDelete(id.clone())));

        actions.append(&edit);
        actions.append(&share);
        actions.append(&delete);

        suffix.append(&actions);

        let indicator = LatencyIndicator::new();
        suffix.append(&indicator.widget);
        widget.add_suffix(&suffix);

        let row = Self {
            node,
            widget,
            latency: Latency::Untested,
            indicator,
            edit,
            share,
            delete,
        };

        row.update(false);

        row
    }

    pub fn update(&self, busy: bool) {
        self.edit.set_sensitive(!busy);
        self.edit.set_tooltip_text(Some(t!("nodes.edit").as_ref()));
        self.share
            .set_tooltip_text(Some(t!("nodes.share").as_ref()));

        self.delete
            .set_tooltip_text(Some(t!("nodes.delete").as_ref()));
        self.delete.set_sensitive(!busy);

        self.indicator.update(&self.latency);
    }
}
