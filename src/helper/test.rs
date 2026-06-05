use crate::app::App;

pub const APP_VERSION: &str = env!("CARGO_PKG_VERSION");
pub const APP_LICENSE: &str = env!("CARGO_PKG_LICENSE");

pub fn app_title(app: &App) -> String {
    format!("{}", app.messages.title)
}

pub fn app_title_with_version(app: &App) -> String {
    format!("{} v{}", app.messages.title, APP_VERSION)
}