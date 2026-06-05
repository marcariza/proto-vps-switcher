use crate::app::App;

use crossterm::terminal::ClearType;
use crossterm::{cursor::MoveTo, queue, style::{Attribute, Color, Print, ResetColor, SetAttribute, SetBackgroundColor, SetForegroundColor}};
use crossterm::terminal::Clear;
use std::io::{Stdout};
use anyhow::Result;

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