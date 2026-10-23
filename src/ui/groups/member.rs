use super::{editor::EditorMsg, identity::node_row};
use crate::{
    api::group::GroupNode,
    ui::latency::{Latency, LatencyIndicator},
};
use adw::prelude::*;
use relm4::{Sender, adw, gtk};
use rust_i18n::t;

pub struct MemberRow {
    pub node: GroupNode,
    pub widget: adw::ActionRow,
    pub latency: Latency,
    indicator: LatencyIndicator,
    remove: gtk::Button,
}

impl MemberRow {
    pub fn new(node: GroupNode, input: &Sender<EditorMsg>) -> Self {
        let widget = node_row(&node.name, node.tag.as_deref(), &node.protocol);

        let indicator = LatencyIndicator::new();

        let remove = gtk::Button::builder()
            .icon_name("list-remove-symbolic")
            .css_classes(["flat", "error"])
            .valign(gtk::Align::Center)
            .build();

        let id = node.id.clone();
        let input = input.clone();

        remove.connect_clicked(move |_| input.emit(EditorMsg::RemoveNode(id.clone())));

        widget.add_suffix(&indicator.widget);
        widget.add_suffix(&remove);

        let row = Self {
            node,
            widget,
            latency: Latency::Untested,
            indicator,
            remove,
        };

        row.update();

        row
    }

    pub fn update(&self) {
        self.indicator.update(&self.latency);

        self.remove
            .set_tooltip_text(Some(&t!("groups.remove_node")));
    }
}
