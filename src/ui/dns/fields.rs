use adw::prelude::*;
use gtk::glib;
use relm4::{Sender, adw, gtk};
use rust_i18n::t;

use super::{
    editor::EditorMsg,
    form::{Form, Rule, Rules, Upstream},
};

#[derive(Clone, Copy, Debug)]
pub enum Direction {
    Request,
    Response,
}
#[derive(Clone, Copy, Debug)]
pub enum RuleAction {
    Up,
    Down,
    Delete,
}

pub struct Field {
    pub row: adw::EntryRow,
    pub error: adw::ActionRow,
    key: &'static str,
    required: bool,
}

impl Field {
    fn new(key: &'static str, required: bool, sender: &Sender<EditorMsg>) -> Self {
        let row = adw::EntryRow::builder().use_markup(false).build();
        if required {
            row.add_suffix(
                &gtk::Label::builder()
                    .label("*")
                    .css_classes(["error"])
                    .build(),
            );
        }
        row.upcast_ref::<gtk::Widget>()
            .update_property(&[gtk::accessible::Property::Required(required)]);
        let error = adw::ActionRow::builder()
            .visible(false)
            .use_markup(false)
            .css_classes(["caption", "error"])
            .build();
        row.upcast_ref::<gtk::Widget>()
            .update_relation(&[gtk::accessible::Relation::DescribedBy(&[error
                .upcast_ref::<gtk::Widget>()
                .upcast_ref()])]);
        let input = sender.clone();
        row.connect_changed(move |_| input.emit(EditorMsg::Changed));
        let field = Self {
            row,
            error,
            key,
            required,
        };
        field.retranslate();
        field
    }

    pub fn validate(&self, submitted: bool) -> bool {
        let valid = !self.required || !self.row.text().trim().is_empty();
        let show = submitted && !valid;
        if show {
            self.row.add_css_class("error");
        } else {
            self.row.remove_css_class("error");
        }
        self.error.set_visible(show);
        self.row
            .upcast_ref::<gtk::Widget>()
            .update_state(&[gtk::accessible::State::Invalid(if show {
                gtk::AccessibleInvalidState::True
            } else {
                gtk::AccessibleInvalidState::False
            })]);
        valid
    }

    fn text(&self) -> String {
        self.row.text().trim().to_string()
    }

    fn retranslate(&self) {
        self.row.set_title(&t!(self.key));
        self.error.set_title(&t!("config.required"));
    }
}

struct UpstreamRow {
    row: adw::ExpanderRow,
    name: Field,
    address: Field,
    delete: gtk::Button,
}
struct RuleRow {
    row: adw::ExpanderRow,
    matcher: Field,
    target: Field,
    up: gtk::Button,
    down: gtk::Button,
    delete: gtk::Button,
}

pub struct RuleList {
    group: adw::PreferencesGroup,
    rows: Vec<RuleRow>,
    fallback: Field,
    add: gtk::Button,
    direction: Direction,
}

impl RuleList {
    fn new(direction: Direction, sender: &Sender<EditorMsg>) -> Self {
        let group = adw::PreferencesGroup::new();
        let fallback = Field::new("dns.fallback", false, sender);
        group.add(&fallback.row);
        group.add(&fallback.error);
        let add = gtk::Button::builder()
            .icon_name("list-add-symbolic")
            .valign(gtk::Align::Center)
            .css_classes(["flat"])
            .build();
        let input = sender.clone();
        add.connect_clicked(move |_| input.emit(EditorMsg::AddRule(direction)));
        group.set_header_suffix(Some(&add));
        Self {
            group,
            rows: vec![],
            fallback,
            add,
            direction,
        }
    }

    fn add(&mut self, rule: &Rule, sender: &Sender<EditorMsg>) {
        let row = adw::ExpanderRow::builder().use_markup(false).build();
        let matcher = Field::new("dns.matcher", true, sender);
        let target = Field::new("dns.target", true, sender);
        matcher.row.set_text(&rule.matcher);
        target.row.set_text(&rule.target);
        row.add_row(&matcher.row);
        row.add_row(&matcher.error);
        row.add_row(&target.row);
        row.add_row(&target.error);
        let popover = gtk::Popover::new();
        let actions = gtk::Box::new(gtk::Orientation::Vertical, 0);
        let [up, down, delete] =
            [RuleAction::Up, RuleAction::Down, RuleAction::Delete].map(|action| {
                let button = gtk::Button::builder().css_classes(["flat"]).build();
                if matches!(action, RuleAction::Delete) {
                    button.add_css_class("destructive-action");
                }
                let input = sender.clone();
                let direction = self.direction;
                button.connect_clicked(gtk::glib::clone!(
                    #[weak]
                    popover,
                    #[weak]
                    row,
                    move |_| {
                        popover.popdown();
                        input.emit(EditorMsg::RuleAction(direction, row.clone(), action));
                    }
                ));
                actions.append(&button);
                button
            });
        popover.set_child(Some(&actions));
        row.add_suffix(
            &gtk::MenuButton::builder()
                .icon_name("view-more-symbolic")
                .valign(gtk::Align::Center)
                .popover(&popover)
                .css_classes(["flat"])
                .build(),
        );
        self.group.remove(&self.fallback.row);
        self.group.remove(&self.fallback.error);
        self.group.add(&row);
        self.group.add(&self.fallback.row);
        self.group.add(&self.fallback.error);
        self.rows.push(RuleRow {
            row,
            matcher,
            target,
            up,
            down,
            delete,
        });
    }

    pub fn act(&mut self, row: &adw::ExpanderRow, action: RuleAction) {
        let index = self.rows.iter().position(|r| &r.row == row).unwrap();
        match action {
            RuleAction::Delete => {
                let removed = self.rows.remove(index);
                self.group.remove(&removed.row);
            }
            RuleAction::Up => self.rows.swap(index, index - 1),
            RuleAction::Down => self.rows.swap(index, index + 1),
        }
        for entry in &self.rows {
            self.group.remove(&entry.row);
        }
        self.group.remove(&self.fallback.row);
        self.group.remove(&self.fallback.error);
        for entry in &self.rows {
            self.group.add(&entry.row);
        }
        self.group.add(&self.fallback.row);
        self.group.add(&self.fallback.error);
    }

    fn load(&mut self, rules: &Rules, sender: &Sender<EditorMsg>) {
        for entry in self.rows.drain(..) {
            self.group.remove(&entry.row);
        }
        for rule in &rules.entries {
            self.add(rule, sender);
        }
        self.fallback.row.set_text(&rules.fallback);
    }
    fn read(&self) -> Rules {
        Rules {
            entries: self
                .rows
                .iter()
                .map(|r| Rule {
                    matcher: r.matcher.text(),
                    target: r.target.text(),
                })
                .collect(),
            fallback: self.fallback.text(),
        }
    }
    fn validate(&self, submitted: bool) -> bool {
        let mut valid = true;
        for r in &self.rows {
            valid &= r.matcher.validate(submitted);
            valid &= r.target.validate(submitted);
        }
        valid
    }
    fn refresh(&self) {
        self.group.set_title(&match self.direction {
            Direction::Request => t!("dns.request_rules"),
            Direction::Response => t!("dns.response_rules"),
        });
        self.group.set_description(Some(&match self.direction {
            Direction::Request => t!("dns.request_help"),
            Direction::Response => t!("dns.response_help"),
        }));
        self.add.set_tooltip_text(Some(&t!("dns.add_rule")));
        self.fallback.retranslate();
        for (index, rule) in self.rows.iter().enumerate() {
            rule.matcher.retranslate();
            rule.target.retranslate();
            let matcher = rule.matcher.text();
            let title = if matcher.is_empty() {
                t!("dns.rule").into_owned()
            } else {
                matcher
            };
            rule.row.set_title(&title);
            rule.row.set_subtitle(&rule.target.text());
            rule.up.set_label(&t!("config.move_up"));
            rule.down.set_label(&t!("config.move_down"));
            rule.delete.set_label(&t!("config.delete"));
            rule.up.set_sensitive(index > 0);
            rule.down.set_sensitive(index + 1 < self.rows.len());
        }
    }
}

pub struct Fields {
    pub page: adw::PreferencesPage,
    pub identity: adw::PreferencesGroup,
    pub name: Field,
    upstream_group: adw::PreferencesGroup,
    upstreams: Vec<UpstreamRow>,
    add: gtk::Button,
    request: RuleList,
    response: RuleList,
}

impl Fields {
    pub fn new(sender: &Sender<EditorMsg>) -> Self {
        let page = adw::PreferencesPage::new();
        let identity = adw::PreferencesGroup::new();
        let name = Field::new("config.name", true, sender);
        identity.add(&name.row);
        identity.add(&name.error);

        let upstream_group = adw::PreferencesGroup::new();
        let add = gtk::Button::builder()
            .icon_name("list-add-symbolic")
            .valign(gtk::Align::Center)
            .css_classes(["flat"])
            .build();
        let input = sender.clone();
        add.connect_clicked(move |_| input.emit(EditorMsg::AddUpstream));
        upstream_group.set_header_suffix(Some(&add));
        page.add(&upstream_group);
        let request = RuleList::new(Direction::Request, sender);
        let response = RuleList::new(Direction::Response, sender);
        page.add(&request.group);
        page.add(&response.group);
        Self {
            page,
            identity,
            name,
            upstream_group,
            upstreams: vec![],
            add,
            request,
            response,
        }
    }

    pub fn add_upstream(&mut self, upstream: &Upstream, sender: &Sender<EditorMsg>) {
        let row = adw::ExpanderRow::builder().use_markup(false).build();
        let name = Field::new("dns.upstream_name", true, sender);
        let address = Field::new("dns.address", true, sender);
        name.row.set_text(&upstream.name);
        address.row.set_text(&upstream.address);
        row.add_row(&name.row);
        row.add_row(&name.error);
        row.add_row(&address.row);
        row.add_row(&address.error);
        let delete = gtk::Button::builder()
            .icon_name("user-trash-symbolic")
            .valign(gtk::Align::Center)
            .css_classes(["flat", "destructive-action"])
            .build();
        let input = sender.clone();
        delete.connect_clicked(gtk::glib::clone!(
            #[weak]
            row,
            move |_| input.emit(EditorMsg::RemoveUpstream(row.clone()))
        ));
        row.add_suffix(&delete);
        self.upstream_group.add(&row);
        self.upstreams.push(UpstreamRow {
            row,
            name,
            address,
            delete,
        });
    }
    pub fn remove_upstream(&mut self, row: &adw::ExpanderRow) {
        let index = self.upstreams.iter().position(|r| &r.row == row).unwrap();
        let removed = self.upstreams.remove(index);
        self.upstream_group.remove(&removed.row);
    }
    pub fn rules(&mut self, direction: Direction) -> &mut RuleList {
        match direction {
            Direction::Request => &mut self.request,
            Direction::Response => &mut self.response,
        }
    }
    pub fn add_rule(&mut self, direction: Direction, sender: &Sender<EditorMsg>) {
        let list = self.rules(direction);
        list.add(
            &Rule {
                matcher: String::new(),
                target: String::new(),
            },
            sender,
        );
        list.rows.last().unwrap().row.set_expanded(true);
        list.rows.last().unwrap().matcher.row.grab_focus();
    }
    pub fn load(&mut self, form: &Form, sender: &Sender<EditorMsg>) {
        for entry in self.upstreams.drain(..) {
            self.upstream_group.remove(&entry.row);
        }
        for upstream in &form.upstreams {
            self.add_upstream(upstream, sender);
        }
        self.request.load(&form.request, sender);
        self.response.load(&form.response, sender);
        self.refresh();
    }
    pub fn read(&self) -> Form {
        Form {
            upstreams: self
                .upstreams
                .iter()
                .map(|r| Upstream {
                    name: r.name.text(),
                    address: r.address.text(),
                })
                .collect(),
            request: self.request.read(),
            response: self.response.read(),
        }
    }
    pub fn validate(&self, submitted: bool) -> bool {
        let mut valid = self.name.validate(submitted);
        for upstream in &self.upstreams {
            valid &= upstream.name.validate(submitted);
            valid &= upstream.address.validate(submitted);
        }
        valid &= self.request.validate(submitted);
        valid &= self.response.validate(submitted);
        valid
    }
    pub fn refresh(&self) {
        self.name.retranslate();
        self.upstream_group.set_title(&t!("dns.upstreams"));
        self.upstream_group
            .set_description(Some(&t!("dns.upstream_help")));
        self.add.set_tooltip_text(Some(&t!("dns.add_upstream")));
        for upstream in &self.upstreams {
            upstream.name.retranslate();
            upstream.address.retranslate();
            let name = upstream.name.text();
            let title = if name.is_empty() {
                t!("dns.upstream").into_owned()
            } else {
                name
            };
            upstream.row.set_title(&title);
            upstream.row.set_subtitle(&upstream.address.text());
            upstream.delete.set_tooltip_text(Some(&t!("config.delete")));
        }
        self.request.refresh();
        self.response.refresh();
    }
}
