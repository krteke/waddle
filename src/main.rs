rust_i18n::i18n!("locales", fallback = "en");

mod app;
mod error;
mod i18n;
mod settings;

use relm4::{RelmApp, gtk::gio};
use settings::{APPLICATION_ID, ServerSettings};

fn main() {
    init_subscriber();

    let installed = gio::SettingsSchemaSource::default();

    let schema = installed
        .as_ref()
        .and_then(|source| source.lookup(APPLICATION_ID, true))
        .unwrap_or_else(|| {
            gio::SettingsSchemaSource::from_directory(env!("OUT_DIR"), installed.as_ref(), true)
                .expect("could not load the compiled Waddle settings schema")
                .lookup(APPLICATION_ID, false)
                .expect("the compiled schema must contain Waddle settings")
        });

    let settings = gio::Settings::new_full(&schema, None::<&gio::SettingsBackend>, None);
    let settings = ServerSettings::load(settings);

    RelmApp::new(APPLICATION_ID).run::<app::App>(settings);

    gio::Settings::sync();
}

fn init_subscriber() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::builder()
                .with_default_directive(tracing::Level::WARN.into())
                .from_env_lossy(),
        )
        .init();
}
