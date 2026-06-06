use std::io::{Stdout, Write};
use std::path::PathBuf;

use anyhow::Result;
use crossterm::cursor::MoveTo;
use crossterm::event::KeyCode;
use crossterm::style::{Color, Print, ResetColor, SetBackgroundColor, SetForegroundColor};
use crossterm::queue;
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode,
};

use crate::app::App;
use crate::config::{AuthConfig, ConnectionConfig, Language};
use crate::helper::comp;
use crate::helper::test::{APP_LICENSE, APP_VERSION, app_title};
use crate::helper::tui::{draw_shell, draw_title, read_key};
use crate::ssh;

static APP_AUTHORS: std::sync::LazyLock<Vec<&'static str>> =
    std::sync::LazyLock::new(comp::parse_authors);

struct MenuItem {
    label: fn(&App) -> &'static str,
    action: fn(&mut Stdout, &mut App) -> Result<()>,
}

fn menu_items() -> Vec<MenuItem> {
    vec![
        MenuItem { label: |app| app.messages.select_connection_menu, action: select_and_connect },
        MenuItem { label: |app| app.messages.add_connection_menu,    action: add_connection },
        MenuItem { label: |app| app.messages.delete_connection_menu, action: delete_connection },
        MenuItem { label: |app| app.messages.change_language_menu,   action: change_language },
        MenuItem { label: |app| app.messages.about_menu,             action: show_app_info },
        MenuItem { label: |app| app.messages.settings_menu,          action: open_settings },
        MenuItem { label: |app| app.messages.exit_menu,              action: |_, _| Ok(()) },
    ]
}

pub fn draw_main_menu(stdout: &mut Stdout, app: &mut App) -> Result<()> {
    let mut selected = 0;
    let items = menu_items();

    loop {
        render_main_menu(stdout, app, &items, selected)?;

        let Some(key) = read_key()? else { continue };

        match key {
            KeyCode::Char('q') | KeyCode::Esc => break,
            KeyCode::Up   | KeyCode::Char('k') => selected = wrap_up(selected, items.len()),
            KeyCode::Down | KeyCode::Char('j') => selected = wrap_down(selected, items.len()),
            KeyCode::Enter => {
                if selected == items.len() - 1 { break; }
                (items[selected].action)(stdout, app)?;
            }
            _ => {}
        }
    }

    Ok(())
}

fn render_main_menu(stdout: &mut Stdout, app: &App, items: &[MenuItem], selected: usize) -> Result<()> {
    draw_shell(stdout, app)?;
    draw_title(stdout, app.messages.main_menu_title)?;

    for (index, item) in items.iter().enumerate() {
        menu_row(stdout, 5 + index as u16, index == selected, (item.label)(app), Color::DarkGreen)?;
    }

    draw_status(stdout, app)?;
    stdout.flush()?;
    Ok(())
}

fn menu_row(stdout: &mut Stdout, row: u16, selected: bool, label: &str, accent: Color) -> Result<()> {
    let marker    = if selected { " *" } else { "  " };
    let foreground = if selected { Color::Black } else { accent };
    let background = if selected { accent } else { Color::Reset };

    queue!(
        stdout,
        MoveTo(0, row),
        SetForegroundColor(foreground),
        SetBackgroundColor(background),
        Print(format!("{marker} {:<40}", truncate(label, 40))),
        ResetColor,
    )?;
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

    let mut selected = app.selected.min(app.store.connections.len().saturating_sub(1));

    loop {
        render_connection_picker(stdout, app, title, selected)?;

        let Some(key) = read_key()? else { continue };

        match key {
            KeyCode::Esc | KeyCode::Char('q') => return Ok(None),
            KeyCode::Up   | KeyCode::Char('k') => selected = wrap_up(selected, app.store.connections.len()),
            KeyCode::Down | KeyCode::Char('j') => selected = wrap_down(selected, app.store.connections.len()),
            KeyCode::Enter => return Ok(Some(selected)),
            _ => {}
        }
    }
}

fn render_connection_picker(stdout: &mut Stdout, app: &App, title: &str, selected: usize) -> Result<()> {
    draw_shell(stdout, app)?;
    draw_title(stdout, title)?;

    queue!(
        stdout,
        MoveTo(0, 5),
        SetForegroundColor(Color::DarkGrey),
        Print(format!(
            "   {:<18} {:<18} {:<8} {:<6} {:<15}",
            app.messages.name_header, app.messages.host_header,
            app.messages.user_header, app.messages.port_header,
            app.messages.auth_header,
        )),
        ResetColor,
        MoveTo(0, 6),
        SetForegroundColor(Color::DarkGrey),
        Print("   ------------------ ------------------ -------- ------ ---------------"),
        ResetColor,
    )?;

    for (index, connection) in app.store.connections.iter().enumerate() {
        render_connection_row(stdout, 7 + index as u16, index == selected, connection, app)?;
    }

    draw_status(stdout, app)?;
    stdout.flush()?;
    Ok(())
}

fn render_connection_row(
    stdout: &mut Stdout,
    row: u16,
    selected: bool,
    connection: &ConnectionConfig,
    app: &App,
) -> Result<()> {
    let marker     = if selected { " >" } else { "  " };
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

fn connect_selected(stdout: &mut Stdout, app: &mut App) -> Result<()> {
    let Some(connection) = app.selected_connection().cloned() else {
        return Ok(());
    };

    suspend_terminal(stdout)?;
    let result = ssh::connect(&connection);
    resume_terminal(stdout)?;

    app.status = Some(match result {
        Ok(_)      => format!("{} {}.", app.messages.con_disconnected, connection.name),
        Err(error) => format!("{}: {error:#}", app.messages.connect_failed),
    });

    Ok(())
}

fn add_connection(stdout: &mut Stdout, app: &mut App) -> Result<()> {
    let Some(name)     = prompt_required(stdout, app, app.messages.name, false)? else { return Ok(()) };
    let Some(host)     = prompt_required(stdout, app, app.messages.host, false)? else { return Ok(()) };
    let Some(user)     = prompt_required(stdout, app, app.messages.user, false)? else { return Ok(()) };
    let Some(port_raw) = prompt_with_default(stdout, app, app.messages.port, "22", false)? else { return Ok(()) };

    let Ok(port) = port_raw.parse::<u16>() else {
        app.status = Some(app.messages.invalid_port.to_owned());
        return Ok(());
    };

    let auth_choices = &["Interactive password", "Stored password", "Key file"];
    let Some(auth_index) = pick_from_list(stdout, app, app.messages.add_title, auth_choices)? else {
        return Ok(());
    };

    let auth = match auth_index {
        0 => AuthConfig::InteractivePassword,
        1 => {
            let Some(password) = prompt_required(stdout, app, app.messages.password, true)? else { return Ok(()) };
            AuthConfig::StoredPassword { password }
        }
        _ => {
            let Some(path) = prompt_required(stdout, app, app.messages.key_path, false)? else { return Ok(()) };
            AuthConfig::KeyFile { path: PathBuf::from(path) }
        }
    };

    app.add_connection(ConnectionConfig { name, host, user, port, auth })?;
    app.status = Some(app.messages.saved.to_owned());
    Ok(())
}

fn change_language(stdout: &mut Stdout, app: &mut App) -> Result<()> {
    let labels = Language::choices().iter().map(|(_, l)| *l).collect::<Vec<_>>();
    let Some(index) = pick_from_list(stdout, app, app.messages.language_title, &labels)? else {
        return Ok(());
    };
    app.change_language(Language::choices()[index].0)
}

fn show_app_info(stdout: &mut Stdout, app: &mut App) -> Result<()> {
    let authors = join_with_and(&APP_AUTHORS, app.messages.and);

    loop {
        draw_shell(stdout, app)?;
        draw_title(stdout, app.messages.about_menu)?;

        queue!(
            stdout,
            MoveTo(0, 5), SetForegroundColor(Color::Cyan),
            Print(app_title(app)),
            ResetColor,
            MoveTo(0, 6), SetForegroundColor(Color::DarkGrey),
            Print(format!("{} {}", app.messages.about_version, APP_VERSION)),
            MoveTo(0, 7),
            Print(format!("{} {}", app.messages.about_authors, authors)),
            MoveTo(0, 8),
            Print(format!("{} {}", app.messages.about_license, APP_LICENSE)),
            ResetColor,
            MoveTo(0, 10), SetForegroundColor(Color::DarkGrey),
            Print(app.messages.any_to_return),
            ResetColor,
        )?;

        stdout.flush()?;
        let Some(_) = read_key()? else { continue };
        break;
    }
    Ok(())
}

fn join_with_and(items: &[&str], and_word: &str) -> String {
    match items.len() {
        0 => String::new(),
        1 => items[0].to_string(),
        2 => format!("{} {} {}", items[0], and_word, items[1]),
        _ => format!("{}, {} {}", items[..items.len() - 1].join(", "), and_word, items[items.len() - 1]),
    }
}

fn open_settings(_stdout: &mut Stdout, _app: &mut App) -> Result<()> {
    Ok(()) // Gotta implement lol
}

fn pick_from_list(stdout: &mut Stdout, app: &App, title: &str, items: &[&str]) -> Result<Option<usize>> {
    let mut selected = 0;

    loop {
        draw_shell(stdout, app)?;
        draw_title(stdout, title)?;

        for (index, item) in items.iter().enumerate() {
            menu_row(stdout, 5 + index as u16, index == selected, item, Color::Cyan)?;
        }
        stdout.flush()?;

        let Some(key) = read_key()? else { continue };

        match key {
            KeyCode::Esc | KeyCode::Char('q') => return Ok(None),
            KeyCode::Up   | KeyCode::Char('k') => selected = wrap_up(selected, items.len()),
            KeyCode::Down | KeyCode::Char('j') => selected = wrap_down(selected, items.len()),
            KeyCode::Enter => return Ok(Some(selected)),
            _ => {}
        }
    }
}

fn prompt_required(stdout: &mut Stdout, app: &mut App, label: &str, secret: bool) -> Result<Option<String>> {
    loop {
        let Some(value) = prompt_text(stdout, app, label, secret)? else { return Ok(None) };

        if !value.trim().is_empty() {
            return Ok(Some(value));
        }
        app.status = Some(app.messages.required_field.to_owned());
    }
}

fn prompt_with_default(stdout: &mut Stdout, app: &App, label: &str, default: &str, secret: bool) -> Result<Option<String>> {
    Ok(prompt_text(stdout, app, label, secret)?.map(|v| {
        if v.trim().is_empty() { default.to_owned() } else { v }
    }))
}

fn prompt_text(stdout: &mut Stdout, app: &App, label: &str, secret: bool) -> Result<Option<String>> {
    let mut value = String::new();

    loop {
        draw_shell(stdout, app)?;
        draw_title(stdout, label)?;

        queue!(
            stdout,
            MoveTo(0, 4), SetBackgroundColor(Color::Black), SetForegroundColor(Color::White),
            Print(if secret { "*".repeat(value.chars().count()) } else { value.clone() }),
            ResetColor,
            MoveTo(0, 6), SetForegroundColor(Color::DarkGrey),
            Print(app.messages.esc_to_cancel),
        )?;
        draw_status(stdout, app)?;
        stdout.flush()?;

        let Some(key) = read_key()? else { continue };

        match key {
            KeyCode::Esc       => return Ok(None),
            KeyCode::Enter     => return Ok(Some(value.trim().to_owned())),
            KeyCode::Backspace => { value.pop(); }
            KeyCode::Char(c)   => value.push(c),
            _ => {}
        }
    }
}

fn wrap_up(selected: usize, len: usize) -> usize {
    if len == 0 { 0 } else if selected == 0 { len - 1 } else { selected - 1 }
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
    use crossterm::cursor::Show;
    use crossterm::terminal::LeaveAlternateScreen;
    crossterm::execute!(stdout, Show, LeaveAlternateScreen)?;
    disable_raw_mode()?;
    Ok(())
}

fn resume_terminal(stdout: &mut Stdout) -> Result<()> {
    use crossterm::cursor::Hide;
    use crossterm::terminal::EnterAlternateScreen;
    enable_raw_mode()?;
    crossterm::execute!(stdout, EnterAlternateScreen, Hide)?;
    Ok(())
}