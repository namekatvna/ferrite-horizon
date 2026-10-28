use std::cell::RefCell;
use std::fmt;
use std::str::FromStr;

use crate::__private::sdk::hud as host;

pub const MAX_LINES: usize = 16;
pub const MAX_LINE_CHARS: usize = 256;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "kebab-case"))]
pub enum Corner {
    #[default]
    TopLeft,
    TopRight,
    BottomLeft,
    BottomRight,
}

impl Corner {
    pub const ALL: [Self; 4] = [
        Self::TopLeft,
        Self::TopRight,
        Self::BottomLeft,
        Self::BottomRight,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Self::TopLeft => "top-left",
            Self::TopRight => "top-right",
            Self::BottomLeft => "bottom-left",
            Self::BottomRight => "bottom-right",
        }
    }

    fn to_host(self) -> host::Corner {
        match self {
            Self::TopLeft => host::Corner::TopLeft,
            Self::TopRight => host::Corner::TopRight,
            Self::BottomLeft => host::Corner::BottomLeft,
            Self::BottomRight => host::Corner::BottomRight,
        }
    }
}

impl fmt::Display for Corner {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ParseCornerError;

impl fmt::Display for ParseCornerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("expected top-left, top-right, bottom-left or bottom-right")
    }
}

impl std::error::Error for ParseCornerError {}

impl FromStr for Corner {
    type Err = ParseCornerError;

    fn from_str(text: &str) -> Result<Self, ParseCornerError> {
        let normalized = text.trim().to_ascii_lowercase().replace(['_', ' '], "-");
        Self::ALL
            .into_iter()
            .find(|c| c.name() == normalized)
            .ok_or(ParseCornerError)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(transparent))]
pub struct Color(u32);

impl Color {
    pub const BLACK: Self = Self(0x000000);
    pub const DARK_BLUE: Self = Self(0x0000AA);
    pub const DARK_GREEN: Self = Self(0x00AA00);
    pub const DARK_AQUA: Self = Self(0x00AAAA);
    pub const DARK_RED: Self = Self(0xAA0000);
    pub const DARK_PURPLE: Self = Self(0xAA00AA);
    pub const GOLD: Self = Self(0xFFAA00);
    pub const GRAY: Self = Self(0xAAAAAA);
    pub const DARK_GRAY: Self = Self(0x555555);
    pub const BLUE: Self = Self(0x5555FF);
    pub const GREEN: Self = Self(0x55FF55);
    pub const AQUA: Self = Self(0x55FFFF);
    pub const RED: Self = Self(0xFF5555);
    pub const LIGHT_PURPLE: Self = Self(0xFF55FF);
    pub const YELLOW: Self = Self(0xFFFF55);
    pub const WHITE: Self = Self(0xFFFFFF);

    pub const fn rgb(r: u8, g: u8, b: u8) -> Self {
        Self(((r as u32) << 16) | ((g as u32) << 8) | b as u32)
    }

    pub const fn hex(rgb: u32) -> Self {
        Self(rgb & 0xFF_FFFF)
    }

    pub const fn to_rgb(self) -> u32 {
        self.0
    }
}

impl Default for Color {
    fn default() -> Self {
        Self::WHITE
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Line {
    pub text: String,
    pub color: Color,
}

impl Line {
    pub fn new(text: impl Into<String>, color: Color) -> Self {
        let mut text = text.into();
        if let Some((cut, _)) = text.char_indices().nth(MAX_LINE_CHARS) {
            text.truncate(cut);
        }
        Self { text, color }
    }
}

impl From<&str> for Line {
    fn from(text: &str) -> Self {
        Self::new(text, Color::WHITE)
    }
}

impl From<String> for Line {
    fn from(text: String) -> Self {
        Self::new(text, Color::WHITE)
    }
}

impl<S: Into<String>> From<(S, Color)> for Line {
    fn from((text, color): (S, Color)) -> Self {
        Self::new(text, color)
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct Panel {
    corner: Corner,
    lines: Vec<Line>,
}

impl Panel {
    pub fn new(corner: Corner) -> Self {
        Self {
            corner,
            lines: Vec::new(),
        }
    }

    pub fn line(mut self, text: impl Into<String>) -> Self {
        self.push(Line::new(text, Color::WHITE));
        self
    }

    pub fn colored(mut self, text: impl Into<String>, color: Color) -> Self {
        self.push(Line::new(text, color));
        self
    }

    pub fn push(&mut self, line: impl Into<Line>) -> &mut Self {
        if self.lines.len() < MAX_LINES {
            self.lines.push(line.into());
        }
        self
    }

    pub fn set_corner(&mut self, corner: Corner) -> &mut Self {
        self.corner = corner;
        self
    }

    pub fn corner(&self) -> Corner {
        self.corner
    }

    pub fn lines(&self) -> &[Line] {
        &self.lines
    }

    pub fn is_empty(&self) -> bool {
        self.lines.is_empty()
    }

    pub fn show(&self) {
        if self.is_empty() {
            clear();
            return;
        }
        let changed = SHOWN.with(|shown| {
            let mut shown = shown.borrow_mut();
            if shown.as_ref() == Some(self) {
                return false;
            }
            *shown = Some(self.clone());
            true
        });
        if changed {
            let lines: Vec<host::Line> = self
                .lines
                .iter()
                .map(|l| host::Line {
                    text: l.text.clone(),
                    color: l.color.to_rgb(),
                })
                .collect();
            host::set_panel(self.corner.to_host(), &lines);
        }
    }
}

thread_local! {
    static SHOWN: RefCell<Option<Panel>> = const { RefCell::new(None) };
}

pub fn clear() {
    let had_panel = SHOWN.with(|shown| shown.borrow_mut().take().is_some());
    if had_panel {
        host::clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn panel_keeps_at_most_sixteen_lines() {
        let mut panel = Panel::new(Corner::TopRight);
        for i in 0..40 {
            panel.push(format!("line {i}"));
        }
        assert_eq!(panel.lines().len(), MAX_LINES);
        assert_eq!(panel.lines()[15].text, "line 15");
    }

    #[test]
    fn long_lines_are_cut_on_a_char_boundary() {
        let line = Line::from("ж".repeat(300));
        assert_eq!(line.text.chars().count(), MAX_LINE_CHARS);
    }

    #[test]
    fn builder_and_conversions() {
        let panel = Panel::new(Corner::BottomLeft)
            .line("plain")
            .colored("warn", Color::GOLD);
        assert_eq!(panel.lines()[0], Line::new("plain", Color::WHITE));
        assert_eq!(panel.lines()[1].color, Color::GOLD);
        assert_eq!(Line::from(("x", Color::RED)).color, Color::RED);
    }

    #[test]
    fn colors() {
        assert_eq!(Color::rgb(0xFF, 0xAA, 0x00), Color::GOLD);
        assert_eq!(Color::hex(0xABCD_EF12).to_rgb(), 0xCDEF12);
    }

    #[test]
    fn corners_parse_loosely() {
        assert_eq!("top-right".parse::<Corner>(), Ok(Corner::TopRight));
        assert_eq!("Bottom_Left".parse::<Corner>(), Ok(Corner::BottomLeft));
        assert!("middle".parse::<Corner>().is_err());
        for corner in Corner::ALL {
            assert_eq!(corner.to_string().parse::<Corner>(), Ok(corner));
        }
    }
}
