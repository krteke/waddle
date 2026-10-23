use super::GroupMsg;
use crate::api::group::Group;
use adw::prelude::*;
use relm4::{Sender, adw, gtk};
use rust_i18n::t;

pub fn group_row(group: &Group, input: &Sender<GroupMsg>) -> adw::ActionRow {
    let row = adw::ActionRow::builder()
        .title(&group.name)
        .subtitle(
            t!(
                "groups.summary",
                policy = group.policy.as_str(),
                nodes = group.nodes.len(),
                subscriptions = group.subscriptions.len()
            )
            .as_ref(),
        )
        .use_markup(false)
        .activatable(true)
        .title_lines(1)
        .subtitle_lines(0)
        .build();

    let id = group.id.clone();
    let open = input.clone();

    row.connect_activated(move |_| open.emit(GroupMsg::Edit(id.clone())));

    let delete = gtk::Button::builder()
        .icon_name("user-trash-symbolic")
        .css_classes(["flat", "error"])
        .valign(gtk::Align::Center)
        .tooltip_text(t!("groups.delete").as_ref())
        .build();

    let id = group.id.clone();
    let input = input.clone();

    delete.connect_clicked(move |_| input.emit(GroupMsg::ConfirmDelete(id.clone())));
    row.add_suffix(&delete);

    row
}
