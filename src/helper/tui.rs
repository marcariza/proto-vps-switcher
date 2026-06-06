use crate::app::App;
use anyhow::Result;
use crossterm::cursor::MoveTo;
use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use crossterm::style::{
    Attribute, Color, Print, ResetColor, SetAttribute, SetBackgroundColor, SetForegroundColor,
};
use crossterm::terminal::{Clear, ClearType};
use crossterm::{queue};
use std::io::Stdout;
use std::time::Duration;

use crate::helper::test::app_title_with_version;

pub fn draw_shell(stdout: &mut Stdout, app: &App) -> Result<()> {
    queue!(
        stdout,
        Clear(ClearType::All),
        MoveTo(0, 0),
        SetBackgroundColor(Color::White),
        SetForegroundColor(Color::Cyan),
        SetAttribute(Attribute::Bold),
        Print(app_title_with_version(app)),
        // Print("Swap test"),
        ResetColor,
        SetAttribute(Attribute::Reset),
        SetForegroundColor(Color::DarkGrey),
    )?;
    Ok(())
}

pub fn draw_title(stdout: &mut Stdout, title: &str) -> Result<()> {
    queue!(
        stdout,
        // Clear(ClearType::All),
        MoveTo(0, 2),
        SetForegroundColor(Color::White),
        SetAttribute(Attribute::Bold),
        Print(title),
        SetAttribute(Attribute::Reset),
        ResetColor,
        MoveTo(0, 3),
        SetForegroundColor(Color::DarkGrey),
        Print("========================"),
        ResetColor,
    )?;

    Ok(())
}

pub fn read_key() -> Result<Option<KeyCode>> {
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