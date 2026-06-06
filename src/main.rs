mod app;
mod config;
mod i18n;
mod ssh;
mod ui;
pub mod helper;
pub mod modules;

use anyhow::Result;

fn main() -> Result<()> {
    let config_path = config::default_config_path()?;
    let store = config::ConnectionStore::load_or_default(&config_path)?;
    let messages = i18n::Messages::for_language(store.language);
    let mut app = app::App::new(config_path, store, messages);

    ui::run(&mut app)
}
