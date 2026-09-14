//! The splash screen: the logo animation, played once before the console draws.

use std::time::Duration;

use ratatui::layout::{Constraint, Flex, Layout, Rect};
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Text};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

use crate::animation::CLI_FRAMES;

/// Time each frame is shown, the pace piramid serve plays the same frames at.
pub const FRAME: Duration = Duration::from_millis(45);

/// Time the finished logo is held before the console draws.
pub const HOLD: Duration = Duration::from_millis(600);

/// The line under the logo.
pub const TAGLINE: &str = "inference engine for RAG";

const ACCENT: Color = Color::Cyan;
const DIM: Color = Color::DarkGray;

/// The animation frames, trimmed to the rows and columns any frame uses.
pub struct Splash {
    frames: Vec<Vec<String>>,
    width: u16,
    height: u16,
}

impl Default for Splash {
    fn default() -> Self {
        Self::new(CLI_FRAMES)
    }
}

impl Splash {
    /// A splash over the given frames, with blank rows and columns shared by every frame removed.
    pub fn new(frames: &[&str]) -> Self {
        let frames = trim(frames);
        let height = frames.first().map_or(0, Vec::len);
        let width = frames
            .iter()
            .flatten()
            .map(|row| row.chars().count())
            .max()
            .unwrap_or(0);
        Self {
            frames,
            width: u16::try_from(width).unwrap_or(u16::MAX),
            height: u16::try_from(height).unwrap_or(u16::MAX),
        }
    }

    /// How many frames play.
    pub fn len(&self) -> usize {
        self.frames.len()
    }

    /// Whether there is nothing to play.
    pub fn is_empty(&self) -> bool {
        self.frames.is_empty()
    }

    /// Columns and rows the logo takes, without the tagline.
    pub fn size(&self) -> (u16, u16) {
        (self.width, self.height)
    }

    /// Whether the logo and its tagline fit in the area.
    pub fn fits(&self, area: Rect) -> bool {
        !self.is_empty() && area.width >= self.width && area.height >= self.height + 2
    }

    /// Draws one frame centred in the frame's area; the last frame carries the tagline.
    pub fn draw(&self, frame: &mut Frame, index: usize) {
        let Some(rows) = self.frames.get(index.min(self.len().saturating_sub(1))) else {
            return;
        };
        let settled = index + 1 >= self.len();
        let [logo, _, tagline] = Layout::vertical([
            Constraint::Length(self.height),
            Constraint::Length(1),
            Constraint::Length(1),
        ])
        .flex(Flex::Center)
        .areas(frame.area());
        let [logo] = Layout::horizontal([Constraint::Length(self.width)])
            .flex(Flex::Center)
            .areas(logo);
        let text = Text::from(
            rows.iter()
                .map(|row| Line::raw(row.as_str()))
                .collect::<Vec<_>>(),
        );
        frame.render_widget(Paragraph::new(text).style(Style::new().fg(ACCENT)), logo);
        if settled {
            frame.render_widget(
                Paragraph::new(TAGLINE)
                    .style(Style::new().fg(DIM))
                    .centered(),
                tagline,
            );
        }
    }
}

/// Every frame split into rows, all the same height, cut to the rows and columns any frame uses.
pub fn trim(frames: &[&str]) -> Vec<Vec<String>> {
    let grid: Vec<Vec<Vec<char>>> = frames
        .iter()
        .map(|frame| frame.lines().map(|row| row.chars().collect()).collect())
        .collect();
    let height = grid.iter().map(Vec::len).max().unwrap_or(0);
    let used = |c: &char| *c != ' ';
    let row_used = |i: usize| {
        grid.iter()
            .any(|f| f.get(i).is_some_and(|r| r.iter().any(used)))
    };
    let col_used = |j: usize| {
        grid.iter()
            .any(|f| f.iter().any(|r| r.get(j).is_some_and(used)))
    };
    let width = grid.iter().flatten().map(Vec::len).max().unwrap_or(0);
    let (Some(top), Some(bottom)) = (
        (0..height).find(|&i| row_used(i)),
        (0..height).rfind(|&i| row_used(i)),
    ) else {
        return frames.iter().map(|_| Vec::new()).collect();
    };
    let left = (0..width).find(|&j| col_used(j)).unwrap_or(0);
    let right = (0..width).rfind(|&j| col_used(j)).unwrap_or(0);
    grid.iter()
        .map(|f| {
            (top..=bottom)
                .map(|i| {
                    let row = f.get(i).map(Vec::as_slice).unwrap_or_default();
                    let cut: String = (left..=right)
                        .map(|j| row.get(j).copied().unwrap_or(' '))
                        .collect();
                    cut.trim_end().to_owned()
                })
                .collect()
        })
        .collect()
}
