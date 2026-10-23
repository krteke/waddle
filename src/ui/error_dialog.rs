use crate::error::ApiError;
use adw::prelude::*;
use relm4::{adw, gtk};
use rust_i18n::t;

pub trait DialogExt {
    fn show_error(&self, heading: &str, source: &ApiError, parent: &impl IsA<gtk::Widget>);
}

impl DialogExt for adw::AlertDialog {
    fn show_error(&self, heading: &str, source: &ApiError, parent: &impl IsA<gtk::Widget>) {
        let details = match source {
            ApiError::Graphql(errors) => errors
                .iter()
                .map(|error| error.message.as_str())
                .collect::<Vec<_>>()
                .join("\n\n"),

            error => {
                let mut details = error.to_string();
                let mut source = std::error::Error::source(error);

                while let Some(error) = source {
                    details.push('\n');
                    details.push_str(&error.to_string());
                    source = error.source();
                }

                details
            }
        };

        let label = gtk::Label::builder()
            .label(details)
            .use_markup(false)
            .selectable(true)
            .wrap(true)
            .wrap_mode(gtk::pango::WrapMode::WordChar)
            .xalign(0.0)
            .build();

        let scroll = gtk::ScrolledWindow::builder()
            .hscrollbar_policy(gtk::PolicyType::Never)
            .propagate_natural_height(true)
            .max_content_height(300)
            .child(&label)
            .build();

        self.set_body("");
        self.set_heading(Some(heading));
        self.set_response_label("close", t!("dialog.close").as_ref());
        self.set_extra_child(Some(&scroll));
        self.present(Some(parent));
    }
}
