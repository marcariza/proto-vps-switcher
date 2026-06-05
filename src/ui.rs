use std::io::{self, Stdout, Write};
use std::path::PathBuf;
use std::time::Duration;

use anyhow::Result;
use crossterm::cursor::{Hide, MoveTo, Show};
use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use crossterm::style::{
    Color, Print, ResetColor, SetBackgroundColor, SetForegroundColor,
};
use crossterm::terminal::{
    EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
};
use crossterm::{execute, queue};

use crate::app::App;
use crate::helper::comp;

use crate::config::{AuthConfig, ConnectionConfig, Language};
use crate::helper::test::{APP_LICENSE, APP_VERSION, app_title};
use crate::helper::tui::{draw_shell, draw_title};
use crate::ssh;

const MENU_LEN: usize = 6;


static APP_AUTHORS: std::sync::LazyLock<Vec<&'static str>> =
    std::sync::LazyLock::new(comp::parse_authors);

#[derive(Clone, Copy)]
enum MainAction {
    SelectConnection,
    AddConnection,
    DeleteConnection,
    ChangeLanguage,
    ShowAppInfo,
    Exit,
}

pub fn run(app: &mut App) -> Result<()> {
    let mut terminal = TerminalSession::enter()?;
    let mut selected = 0;

    loop {
        draw_main_menu(terminal.stdout(), app, selected)?;

        let Some(key) = read_key()? else {
            continue;
        };

        match key {
            KeyCode::Char('q') | KeyCode::Esc => break,
            KeyCode::Up | KeyCode::Char('k') => selected = wrap_up(selected, MENU_LEN),
            KeyCode::Down | KeyCode::Char('j') => selected = wrap_down(selected, MENU_LEN),
            KeyCode::Enter => match main_action(selected) {
                MainAction::SelectConnection => select_and_connect(terminal.stdout(), app)?,
                MainAction::AddConnection => add_connection(terminal.stdout(), app)?,
                MainAction::DeleteConnection => delete_connection(terminal.stdout(), app)?,
                MainAction::ChangeLanguage => change_language(terminal.stdout(), app)?,
                MainAction::ShowAppInfo => show_app_info(terminal.stdout(), app)?,
                MainAction::Exit => break,
            },
            _ => {}
        }
    }

    Ok(())
}

fn draw_main_menu(stdout: &mut Stdout, app: &App, selected: usize) -> Result<()> {
    draw_shell(stdout, app)?;

    draw_title(stdout, app.messages.main_menu_title)?;

    let items = [
        app.messages.select_connection_menu,
        app.messages.add_connection_menu,
        app.messages.delete_connection_menu,
        app.messages.change_language_menu,
        app.messages.about_menu,
        app.messages.exit_menu,
    ];

    for (index, item) in items.iter().enumerate() {
        draw_menu_row(
            stdout,
            5 + index as u16,
            index == selected,
            item,
            Color::DarkGreen,
        )?;
    }

    draw_status(stdout, app)?;
    stdout.flush()?;
    Ok(())
}

fn draw_status(stdout: &mut Stdout, app: &App) -> Result<()> {
    if let Some(status) = &app.status {
        queue!(
            stdout,
            MoveTo(0, 22),
            SetForegroundColor(Color::Yellow),
            Print(status),
            ResetColor,
        )?;
    }
    Ok(())
}

fn draw_menu_row(
    stdout: &mut Stdout,
    row: u16,
    selected: bool,
    label: &str,
    accent: Color,
) -> Result<()> {
    let marker = if selected { " *" } else { "  " };
    let foreground = if selected { Color::Black } else { accent };
    let background = if selected { accent } else { Color::Reset };

    queue!(
        stdout,
        MoveTo(0, row),
        SetForegroundColor(foreground),
        SetBackgroundColor(background),
        Print(format!("{marker} {:<30}", truncate(label, 40))),
        ResetColor,
    )?;

    Ok(())
}

fn draw_connection_picker(
    stdout: &mut Stdout,
    app: &App,
    title: &str,
    selected: usize,
) -> Result<()> {
    draw_shell(stdout, app)?;

    draw_title(stdout, title)?;

    queue!(
        stdout,
        MoveTo(0, 5),
        SetForegroundColor(Color::DarkGrey),
        Print(format!(
            "  {:<18} {:<18} {:<8} {:<6} {:<15}",
            app.messages.name_header,
            app.messages.host_header,
            app.messages.user_header,
            app.messages.port_header,
            app.messages.auth_header,
        )),
        ResetColor,
        MoveTo(0, 6),
        SetForegroundColor(Color::DarkGrey),
        Print("  ------------------ ------------------ -------- ------ ---------------"),
        ResetColor,
    )?;

    for (index, connection) in app.store.connections.iter().enumerate() {
        draw_connection_row(stdout, 7 + index as u16, index == selected, connection, app)?;
    }

    draw_status(stdout, app)?;
    stdout.flush()?;
    Ok(())
}

fn draw_connection_row(
    stdout: &mut Stdout,
    row: u16,
    selected: bool,
    connection: &ConnectionConfig,
    app: &App,
) -> Result<()> {
    let marker = if selected { ">" } else { " " };
    let foreground = if selected { Color::Black } else { Color::White };
    let background = if selected { Color::Green } else { Color::Reset };

    queue!(
        stdout,
        MoveTo(0, row),
        SetForegroundColor(foreground),
        SetBackgroundColor(background),
        Print(format!(
            "{marker} {:<18} {:<18} {:<8} {:<6} {:<15}",
            truncate(&connection.name, 18),
            truncate(&connection.host, 18),
            truncate(&connection.user, 8),
            connection.port,
            truncate(app.messages.auth_label(&connection.auth), 15),
        )),
        ResetColor,
    )?;

    Ok(())
}

fn select_and_connect(stdout: &mut Stdout, app: &mut App) -> Result<()> {
    let Some(index) = pick_connection(stdout, app, app.messages.select_title)? else {
        return Ok(());
    };

    app.selected = index;
    connect_selected(stdout, app)
}

fn delete_connection(stdout: &mut Stdout, app: &mut App) -> Result<()> {
    let Some(index) = pick_connection(stdout, app, app.messages.delete_title)? else {
        return Ok(());
    };

    app.selected = index;
    app.delete_selected()
}

fn pick_connection(stdout: &mut Stdout, app: &mut App, title: &str) -> Result<Option<usize>> {
    if app.store.connections.is_empty() {
        app.status = Some(app.messages.no_connections.to_owned());
        return Ok(None);
    }

    let mut selected = app
        .selected
        .min(app.store.connections.len().saturating_sub(1));

    loop {
        draw_connection_picker(stdout, app, title, selected)?;

        let Some(key) = read_key()? else {
            continue;
        };

        match key {
            KeyCode::Esc | KeyCode::Char('q') => return Ok(None),
            KeyCode::Up | KeyCode::Char('k') => {
                selected = wrap_up(selected, app.store.connections.len())
            }
            KeyCode::Down | KeyCode::Char('j') => {
                selected = wrap_down(selected, app.store.connections.len())
            }
            KeyCode::Enter => return Ok(Some(selected)),
            _ => {}
        }
    }
}

fn add_connection(stdout: &mut Stdout, app: &mut App) -> Result<()> {
    let Some(name) = prompt_required(
        stdout,
        app,
        // app.messages.add_title,
        app.messages.name,
        false,
    )?
    else {
        return Ok(());
    };
    let Some(host) = prompt_required(
        stdout,
        app,
        // app.messages.add_title,
        app.messages.host,
        false,
    )?
    else {
        return Ok(());
    };
    let Some(user) = prompt_required(
        stdout,
        app,
        // app.messages.add_title,
        app.messages.user,
        false,
    )?
    else {
        return Ok(());
    };
    let Some(port_raw) = prompt_with_default(
        stdout,
        app,
        // app.messages.add_title,
        app.messages.port,
        "22",
        false,
    )?
    else {
        return Ok(());
    };
    let Ok(port) = port_raw.parse::<u16>() else {
        app.status = Some(app.messages.invalid_port.to_owned());
        return Ok(());
    };

    let Some(auth_index) = pick_from_list(
        stdout,
        app,
        app.messages.add_title,
        &["Interactive password", "Stored password", "Key file"],
    )?
    else {
        return Ok(());
    };

    let auth = match auth_index {
        0 => AuthConfig::InteractivePassword,
        1 => {
            let Some(password) = prompt_required(
                stdout,
                app,
                // app.messages.add_title,
                app.messages.password,
                true,
            )?
            else {
                return Ok(());
            };
            AuthConfig::StoredPassword { password }
        }
        _ => {
            let Some(path) = prompt_required(
                stdout,
                app,
                // app.messages.add_title,
                app.messages.key_path,
                false,
            )?
            else {
                return Ok(());
            };
            AuthConfig::KeyFile {
                path: PathBuf::from(path),
            }
        }
    };

    app.add_connection(ConnectionConfig {
        name,
        host,
        user,
        port,
        auth,
    })?;
    app.status = Some(app.messages.saved.to_owned());
    Ok(())
}

fn change_language(stdout: &mut Stdout, app: &mut App) -> Result<()> {
    let labels = Language::choices()
        .iter()
        .map(|(_, label)| *label)
        .collect::<Vec<_>>();
    let Some(index) = pick_from_list(stdout, app, app.messages.language_title, &labels)? else {
        return Ok(());
    };

    let language = Language::choices()[index].0;
    app.change_language(language)
}

fn join_with_and(items: &[&str], and_word: &str) -> String {
    match items.len() {
        0 => String::new(),
        1 => items[0].to_string(),
        2 => format!("{} {} {}", items[0], and_word, items[1]),
        _ => {
            let mut result = items[..items.len() - 1].join(", ");
            result.push_str(&format!(" {} {}", and_word, items[items.len() - 1]));
            result
        }
    }
}

fn show_app_info(stdout: &mut Stdout, app: &mut App) -> Result<()> {
    loop {
        draw_shell(stdout, app)?;

        draw_title(stdout, app.messages.about_menu)?;

        let authors_string = join_with_and(&APP_AUTHORS, app.messages.and);

        queue!(
            stdout,
            MoveTo(0, 5),
            ResetColor,
            SetForegroundColor(Color::Cyan),
            Print(app_title(app)),
            ResetColor,
            MoveTo(0, 6),
            SetForegroundColor(Color::DarkGrey),
            Print(format!("{} {}", app.messages.about_version, APP_VERSION)),

            MoveTo(0, 7),
            Print(format!("{} {}", app.messages.about_authors, authors_string)),

            MoveTo(0, 8),
            Print(format!("{} {}", app.messages.about_license, APP_LICENSE)),

            ResetColor,
            MoveTo(0, 10),
            SetForegroundColor(Color::DarkGrey),
            Print(app.messages.any_to_return),
            ResetColor,
        )?;

        stdout.flush()?;

        let Some(_) = read_key()? else {
            continue;
        };

        break;
    }

    Ok(())
}

fn pick_from_list(
    stdout: &mut Stdout,
    app: &App,
    title: &str,
    items: &[&str],
) -> Result<Option<usize>> {
    let mut selected = 0;

    loop {
        draw_shell(stdout, app)?;

        draw_title(stdout, title)?;

        // queue!(
        //     stdout,
        //     MoveTo(0, 4),
        //     SetForegroundColor(Color::White),
        //     SetAttribute(Attribute::Bold),
        //     Print(title),
        //     SetAttribute(Attribute::Reset),
        //     ResetColor,
        // )?;

        for (index, item) in items.iter().enumerate() {
            draw_menu_row(
                stdout,
                5 + index as u16,
                index == selected,
                item,
                Color::Cyan,
            )?;
        }

        stdout.flush()?;

        let Some(key) = read_key()? else {
            continue;
        };

        match key {
            KeyCode::Esc | KeyCode::Char('q') => return Ok(None),
            KeyCode::Up | KeyCode::Char('k') => selected = wrap_up(selected, items.len()),
            KeyCode::Down | KeyCode::Char('j') => selected = wrap_down(selected, items.len()),
            KeyCode::Enter => return Ok(Some(selected)),
            _ => {}
        }
    }
}

fn prompt_required(
    stdout: &mut Stdout,
    app: &mut App,
    // title: &str,
    label: &str,
    secret: bool,
) -> Result<Option<String>> {
    loop {
        let Some(value) = prompt_text(stdout,
            app,
            // title,
            label,
            // None,
            secret
        )? else {
            return Ok(None);
        };

        if !value.trim().is_empty() {
            return Ok(Some(value));
        }

        app.status = Some(app.messages.required_field.to_owned());
    }
}

fn prompt_with_default(
    stdout: &mut Stdout,
    app: &App,
    // title: &str,
    label: &str,
    default: &str,
    secret: bool,
) -> Result<Option<String>> {
    let value = prompt_text(stdout,
        app,
        // title,
        label,
        // Some(default),
    secret)?;
    Ok(value.map(|value| {
        if value.trim().is_empty() {
            default.to_owned()
        } else {
            value
        }
    }))
}

fn prompt_text(
    stdout: &mut Stdout,
    app: &App,
    // title: &str,
    label: &str,
    // default: Option<&str>,
    secret: bool,
) -> Result<Option<String>> {
    let mut value = String::new();

    loop {
        draw_shell(stdout, app)?;

        draw_title(stdout, label)?;

        queue!(
            stdout,
            MoveTo(0, 4),
            SetBackgroundColor(Color::Black),
            SetForegroundColor(Color::White),
            Print(if secret {
                "*".repeat(value.chars().count())
            } else {
                value.clone()
            }),
            ResetColor,

            MoveTo(0, 6),
            SetForegroundColor(Color::DarkGrey),
            Print(app.messages.esc_to_cancel)
        )?;
        draw_status(stdout, app)?;
        stdout.flush()?;

        let Some(key) = read_key()? else {
            continue;
        };

        match key {
            KeyCode::Esc => return Ok(None),
            KeyCode::Enter => return Ok(Some(value.trim().to_owned())),
            KeyCode::Backspace => {
                value.pop();
            }
            KeyCode::Char(character) => value.push(character),
            _ => {}
        }
    }
}

fn connect_selected(stdout: &mut Stdout, app: &mut App) -> Result<()> {
    let Some(connection) = app.selected_connection().cloned() else {
        return Ok(());
    };

    suspend_terminal(stdout)?;
    let result = ssh::connect(&connection);
    resume_terminal(stdout)?;

    match result {
        Ok(_) => {
            app.status = Some(format!(
                "{} {}.",
                app.messages.con_disconnected, connection.name
            ));
        }
        Err(error) => {
            app.status = Some(format!("{}: {error:#}", app.messages.connect_failed));
        }
    }

    Ok(())
}

fn read_key() -> Result<Option<KeyCode>> {
    if !event::poll(Duration::from_millis(250))? {
        return Ok(None);
    }

    let Event::Key(key) = event::read()? else {
        return Ok(None);
    };

    if key.kind != KeyEventKind::Press {
        return Ok(None);
    }

    Ok(Some(key.code))
}

fn main_action(selected: usize) -> MainAction {
    match selected {
        0 => MainAction::SelectConnection,
        1 => MainAction::AddConnection,
        2 => MainAction::DeleteConnection,
        3 => MainAction::ChangeLanguage,
        4 => MainAction::ShowAppInfo,
        _ => MainAction::Exit,
    }
}

fn wrap_up(selected: usize, len: usize) -> usize {
    if len == 0 {
        0
    } else if selected == 0 {
        len - 1
    } else {
        selected - 1
    }
}

fn wrap_down(selected: usize, len: usize) -> usize {
    if len == 0 { 0 } else { (selected + 1) % len }
}

fn truncate(value: &str, max_chars: usize) -> String {
    let mut truncated = value.chars().take(max_chars).collect::<String>();
    if value.chars().count() > max_chars && max_chars > 1 {
        truncated.pop();
        truncated.push('~');
    }
    truncated
}

fn suspend_terminal(stdout: &mut Stdout) -> Result<()> {
    execute!(stdout, Show, LeaveAlternateScreen)?;
    disable_raw_mode()?;
    Ok(())
}

fn resume_terminal(stdout: &mut Stdout) -> Result<()> {
    enable_raw_mode()?;
    execute!(stdout, EnterAlternateScreen, Hide)?;
    Ok(())
}

struct TerminalSession {
    stdout: Stdout,
}

impl TerminalSession {
    fn enter() -> Result<Self> {
        let mut stdout = io::stdout();
        enable_raw_mode()?;
        execute!(stdout, EnterAlternateScreen, Hide)?;
        Ok(Self { stdout })
    }

    fn stdout(&mut self) -> &mut Stdout {
        &mut self.stdout
    }
}

impl Drop for TerminalSession {
    fn drop(&mut self) {
        let _ = execute!(self.stdout, Show, LeaveAlternateScreen);
        let _ = disable_raw_mode();
    }
}
