use gpui::{App, Context, Font, Hsla, Pixels};

use super::{colors::Ink, view::Terminal};
use crate::theme::ActiveTheme;

/// Per-terminal typography and colors, independent of the surrounding app theme.
#[derive(Clone, Debug, PartialEq)]
pub struct Appearance {
    pub font: Font,
    pub font_size: Pixels,
    pub line_height: Pixels,
    pub foreground: Hsla,
    pub background: Hsla,
    pub ansi: [Hsla; 16],
    pub cursor: Hsla,
    pub selection: Hsla,
}

impl Terminal {
    /// Overrides theme defaults; returns an error for nonpositive or nonfinite font and line sizes.
    pub fn set_appearance(
        &mut self,
        appearance: Appearance,
        cx: &mut Context<Self>,
    ) -> anyhow::Result<()> {
        anyhow::ensure!(
            [appearance.font_size, appearance.line_height]
                .into_iter()
                .all(|value| value > Pixels::ZERO && f32::from(value).is_finite()),
            "terminal: font size and line height must be positive and finite"
        );
        if self.appearance.as_ref() != Some(&appearance) {
            self.appearance = Some(appearance);
            cx.notify();
        }
        Ok(())
    }

    pub(super) fn ink(&self, cx: &App) -> Ink {
        self.appearance.as_ref().map_or_else(
            || Ink::new(&cx.theme().colors),
            |style| Ink {
                ansi: style.ansi,
                fg: style.foreground,
                bg: style.background,
            },
        )
    }
}

#[cfg(all(test, feature = "test-support"))]
mod tests;
