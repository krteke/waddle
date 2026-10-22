use super::editor::EditorMsg;
use adw::prelude::*;
use relm4::{Sender, adw, gtk};
use rust_i18n::t;
use std::{cell::RefCell, rc::Rc};

pub struct InputList(Rc<List>);

struct List {
    group: adw::PreferencesGroup,
    error: gtk::Label,
    add: gtk::Button,
    key: &'static str,
    required: bool,
    ordered: bool,
    items: RefCell<Vec<Item>>,
    sender: Sender<EditorMsg>,
}

struct Item {
    entry: adw::EntryRow,
    error: adw::ActionRow,
    remove: gtk::Button,
    ordering: Option<(gtk::Button, gtk::MenuButton, gtk::Button, gtk::Button)>,
}

impl InputList {
    pub fn new(
        key: &'static str,
        required: bool,
        ordered: bool,
        sender: &Sender<EditorMsg>,
    ) -> Self {
        let group = adw::PreferencesGroup::new();

        let error = gtk::Label::builder()
            .xalign(0.0)
            .wrap(true)
            .css_classes(["error", "caption"])
            .visible(false)
            .build();

        let add = gtk::Button::builder()
            .icon_name("list-add-symbolic")
            .valign(gtk::Align::Start)
            .css_classes(["flat"])
            .build();

        let header = gtk::Box::new(gtk::Orientation::Horizontal, 0);
        header.set_valign(gtk::Align::Start);

        let required_mark = gtk::Label::builder()
            .label("*")
            .css_classes(["error"])
            .visible(required)
            .build();

        header.append(&required_mark);
        header.append(&add);

        group.set_header_suffix(Some(&header));
        group.add(&error);

        let result = Self(Rc::new(List {
            group,
            error,
            add,
            key,
            required,
            ordered,
            items: RefCell::new(Vec::new()),
            sender: sender.clone(),
        }));

        let weak = Rc::downgrade(&result.0);

        result.0.add.connect_clicked(move |_| {
            let list = InputList(weak.upgrade().unwrap());
            list.append("");

            list.0.items.borrow().last().unwrap().entry.grab_focus();
            list.0.sender.emit(EditorMsg::Changed);
        });

        result.retranslate();

        result
    }

    pub fn group(&self) -> &adw::PreferencesGroup {
        &self.0.group
    }

    pub fn load(&self, values: &[String]) {
        self.0.items.borrow_mut().iter().for_each(|item| {
            self.0.group.remove(&item.entry);
            self.0.group.remove(&item.error);
        });

        values.iter().for_each(|v| {
            self.append(v);
        });

        self.clear_errors();
    }

    pub fn values(&self) -> Vec<String> {
        self.0
            .items
            .borrow()
            .iter()
            .map(|item| item.entry.text().trim().to_owned())
            .collect()
    }

    fn append(&self, value: &str) {
        let entry = adw::EntryRow::builder()
            .text(value)
            .use_markup(false)
            .build();

        entry
            .upcast_ref::<gtk::Widget>()
            .update_property(&[gtk::accessible::Property::Required(true)]);

        let error = adw::ActionRow::builder()
            .css_classes(["caption", "error"])
            .use_markup(false)
            .title_lines(0)
            .visible(false)
            .activatable(false)
            .build();

        entry.upcast_ref::<gtk::Widget>().update_relation(&[
            gtk::accessible::Relation::DescribedBy(&[error
                .upcast_ref::<gtk::Widget>()
                .upcast_ref()]),
        ]);

        let remove = gtk::Button::builder()
            .valign(gtk::Align::Center)
            .css_classes(["flat"])
            .build();

        let weak = Rc::downgrade(&self.0);
        let item_row = entry.downgrade();

        remove.connect_clicked(move |_| {
            let list = InputList(weak.upgrade().unwrap());
            let row = item_row.upgrade().unwrap();

            let index = list
                .0
                .items
                .borrow()
                .iter()
                .position(|item| item.entry == row)
                .unwrap();

            let item = list.0.items.borrow_mut().remove(index);

            list.0.group.remove(&item.entry);
            list.0.group.remove(&item.error);
            list.retranslate();

            let items = list.0.items.borrow();

            if let Some(item) = items.get(index).or_else(|| items.last()) {
                item.entry.grab_focus();
            } else {
                list.0.add.grab_focus();
            }

            list.0.sender.emit(EditorMsg::Changed);
        });

        let ordering = if self.0.ordered {
            remove.add_css_class("destructive-action");

            let handle = gtk::Button::builder()
                .icon_name("list-drag-handle-symbolic")
                .css_classes(["flat"])
                .focusable(false)
                .valign(gtk::Align::Center)
                .build();

            handle.set_cursor_from_name(Some("grab"));
            entry.add_prefix(&handle);

            let more = gtk::MenuButton::builder()
                .icon_name("view-more-symbolic")
                .valign(gtk::Align::Center)
                .css_classes(["flat"])
                .build();

            let popover = gtk::Popover::new();
            let menu = gtk::Box::new(gtk::Orientation::Vertical, 0);

            let up = gtk::Button::builder().css_classes(["flat"]).build();
            let down = gtk::Button::builder().css_classes(["flat"]).build();

            [(&up, -1), (&down, 1)]
                .into_iter()
                .for_each(|(button, offset)| {
                    let weak = Rc::downgrade(&self.0);

                    let item_row = entry.downgrade();
                    let popover = popover.downgrade();

                    button.connect_clicked(move |_| {
                        popover.upgrade().unwrap().popdown();

                        let list = InputList(weak.upgrade().unwrap());
                        let row = item_row.upgrade().unwrap();

                        let index = list
                            .0
                            .items
                            .borrow()
                            .iter()
                            .position(|item| item.entry == row)
                            .unwrap();

                        list.reorder(index, (index as isize + offset) as usize);
                    });
                });

            menu.append(&up);
            menu.append(&down);
            menu.append(&remove);

            popover.set_child(Some(&menu));
            more.set_popover(Some(&popover));
            entry.add_suffix(&more);

            let drag = gtk::DragSource::builder()
                .actions(gtk::gdk::DragAction::MOVE)
                .build();

            let item_row = entry.downgrade();

            drag.connect_prepare(move |_, _, _| {
                Some(gtk::gdk::ContentProvider::for_value(
                    &item_row
                        .upgrade()
                        .unwrap()
                        .upcast::<gtk::ListBoxRow>()
                        .to_value(),
                ))
            });

            let item_row = entry.downgrade();

            drag.connect_drag_begin(move |source, _| {
                source.set_icon(
                    Some(&gtk::WidgetPaintable::new(Some(
                        &item_row.upgrade().unwrap(),
                    ))),
                    0,
                    0,
                )
            });

            handle.add_controller(drag);

            let drop =
                gtk::DropTarget::new(gtk::ListBoxRow::static_type(), gtk::gdk::DragAction::MOVE);
            drop.set_preload(true);

            let weak = Rc::downgrade(&self.0);
            let target_row = entry.downgrade();

            drop.connect_drop(move |_, value, _, y| {
                let source = value.get::<gtk::ListBoxRow>().unwrap();
                let list = InputList(weak.upgrade().unwrap());

                let target = target_row.upgrade().unwrap();

                if source.parent() != target.parent() {
                    return false;
                }

                let items = list.0.items.borrow();

                let from = items
                    .iter()
                    .position(|item| item.entry.upcast_ref::<gtk::ListBoxRow>() == &source)
                    .unwrap();

                let mut to = items.iter().position(|item| item.entry == target).unwrap()
                    + usize::from(y >= target.height() as f64 / 2.0);

                std::mem::drop(items);

                if from < to {
                    to -= 1;
                }

                list.reorder(from, to);

                true
            });

            entry.add_controller(drop);

            Some((handle, more, up, down))
        } else {
            remove.set_icon_name("edit-delete-symbolic");
            entry.add_suffix(&remove);

            None
        };

        let sender = self.0.sender.clone();
        entry.connect_changed(move |_| sender.emit(EditorMsg::Changed));

        self.0.group.add(&entry);
        self.0.group.add(&error);

        self.0.items.borrow_mut().push(Item {
            entry,
            error,
            remove,
            ordering,
        });

        self.retranslate();
    }

    fn reorder(&self, from: usize, to: usize) {
        let mut items = self.0.items.borrow_mut();
        let item = items.remove(from);

        items.insert(to, item);

        items.iter().for_each(|item| {
            self.0.group.remove(&item.entry);
            self.0.group.remove(&item.error);
        });

        items.iter().for_each(|item| {
            self.0.group.add(&item.entry);
            self.0.group.add(&item.error);
        });

        items[to].entry.grab_focus();

        drop(items);

        self.retranslate();
        self.0.sender.emit(EditorMsg::Changed);
    }

    pub fn retranslate(&self) {
        let title = t!(format!("config.{}", self.0.key));

        self.0.group.set_title(&title);
        self.0
            .group
            .set_description(Some(&t!(format!("config_help.{}", self.0.key))));

        self.0.add.set_tooltip_text(Some(&t!("config.add_item")));
        self.0.error.set_text(&t!("config.required"));

        let items = self.0.items.borrow();

        items.iter().enumerate().for_each(|(i, item)| {
            let label = t!("config.list_item", field = title.as_ref(), index = i + 1);

            item.entry.set_tooltip_text(Some(&label));
            item.entry.set_title(&label);

            item.entry
                .upcast_ref::<gtk::Widget>()
                .update_property(&[gtk::accessible::Property::Label(&label)]);

            item.error.set_title(&t!("config.required"));
            item.remove
                .set_tooltip_text(Some(&t!("config.remove_item")));

            if let Some((handle, more, up, down)) = &item.ordering {
                handle.set_tooltip_text(Some(&t!("config.drag_item")));
                more.set_tooltip_text(Some(&t!("config.item_actions")));

                item.remove.set_label(&t!("config.remove_item"));

                up.set_label(&t!("config.move_up"));
                down.set_label(&t!("config.move_down"));

                up.set_sensitive(i > 0);
                down.set_sensitive(i + 1 < items.len());
            }
        });
    }

    pub fn validate(&self) -> bool {
        let items = self.0.items.borrow();
        let empty = self.0.required && items.is_empty();

        self.0.error.set_visible(empty);

        let mut valid = !empty;

        items.iter().for_each(|item| {
            let invalid = item.entry.text().trim().is_empty();

            item.error.set_visible(invalid);

            if invalid {
                item.entry.add_css_class("error");
            } else {
                item.entry.remove_css_class("error");
            }

            item.entry.upcast_ref::<gtk::Widget>().update_state(&[
                gtk::accessible::State::Invalid(if invalid {
                    gtk::AccessibleInvalidState::True
                } else {
                    gtk::AccessibleInvalidState::False
                }),
            ]);

            valid &= !invalid;
        });

        valid
    }

    pub fn focus_error(&self) {
        if let Some(item) = self
            .0
            .items
            .borrow()
            .iter()
            .find(|item| item.error.is_visible())
        {
            item.entry.grab_focus();
        } else {
            self.0.add.grab_focus();
        }
    }

    pub fn clear_errors(&self) {
        self.0.error.set_visible(false);

        self.0.items.borrow().iter().for_each(|item| {
            item.error.set_visible(false);
            item.entry.remove_css_class("error");

            item.entry.upcast_ref::<gtk::Widget>().update_state(&[
                gtk::accessible::State::Invalid(gtk::AccessibleInvalidState::False),
            ]);
        });
    }
}
