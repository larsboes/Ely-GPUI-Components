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
    /// Overrides the terminal's theme defaults.
    ///
    /// # Errors
    /// Returns an error for nonpositive or nonfinite font and line sizes.
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
mod tests {
    use super::*;
    use crate::{terminal::RemoteTransport, theme::Theme};
    use gpui::{AppContext, TestAppContext, font, px, rgb};

    #[gpui::test]
    fn per_terminal_style_reaches_cells_and_color_queries(cx: &mut TestAppContext) {
        cx.update(Theme::init);
        let (view, cx) = cx.add_window_view(|_, cx| {
            let mut terminal =
                Terminal::remote(20, 3, RemoteTransport::new(|_| Ok(()), |_, _| Ok(())), cx);
            terminal
                .set_appearance(
                    Appearance {
                        font: font("Menlo"),
                        font_size: px(17.),
                        line_height: px(24.),
                        foreground: rgb(0xabcdef).into(),
                        background: rgb(0x101010).into(),
                        ansi: [rgb(0x123456).into(); 16],
                        cursor: rgb(0xff0000).into(),
                        selection: rgb(0x111111).into(),
                    },
                    cx,
                )
                .unwrap();
            terminal.feed_remote(b"\x1b[31mx", cx).unwrap();
            terminal
        });
        cx.run_until_parked();
        view.read_with(cx, |terminal, cx| {
            assert_eq!(terminal.cell.height, px(24.));
            assert_eq!(terminal.shown.rows[0].runs[0].1.fg, rgb(0x123456).into());
            assert_eq!(
                terminal.ink(cx).query(256, terminal.term.lock().colors()),
                vte::ansi::Rgb {
                    r: 0xab,
                    g: 0xcd,
                    b: 0xef
                }
            );
        });
        view.update(cx, |terminal, cx| {
            let mut invalid = terminal.appearance.clone().unwrap();
            invalid.font_size = Pixels::ZERO;
            assert!(terminal.set_appearance(invalid, cx).is_err());
            assert_eq!(terminal.appearance.as_ref().unwrap().font_size, px(17.));
        });
    }

    #[gpui::test]
    fn transport_errors_are_visible_in_the_terminal(cx: &mut TestAppContext) {
        cx.update(Theme::init);
        let (view, cx) = cx.add_window_view(|_, cx| {
            Terminal::remote(
                20,
                3,
                RemoteTransport::new(|_| anyhow::bail!("offline"), |_, _| Ok(())),
                cx,
            )
        });
        view.update(cx, |terminal, cx| terminal.write(b"x".to_vec(), cx));
        cx.run_until_parked();
        assert!(cx.debug_bounds("terminal-transport-error").is_some());
    }

    #[gpui::test]
    fn idle_alternate_screen_wheel_failure_draws_an_error(cx: &mut TestAppContext) {
        use gpui::{Modifiers, ScrollDelta, ScrollEvent, ScrollSource, TouchPhase, point};
        cx.update(Theme::init);
        let (view, cx) = cx.add_window_view(|_, cx| {
            let mut terminal = Terminal::remote(
                20,
                3,
                RemoteTransport::new(|_| anyhow::bail!("offline"), |_, _| Ok(())),
                cx,
            );
            terminal.feed_remote(b"\x1b[?1049h\x1b[?1007h", cx).unwrap();
            terminal
        });
        cx.run_until_parked();
        assert!(cx.debug_bounds("terminal-transport-error").is_none());
        let position = view.read_with(cx, |terminal, _| terminal.origin + point(px(10.), px(10.)));
        cx.simulate_event(ScrollEvent {
            source: ScrollSource::Pointer,
            position,
            delta: ScrollDelta::Lines(point(0., 1.)),
            modifiers: Modifiers::default(),
            touch_phase: TouchPhase::Moved,
        });
        cx.run_until_parked();
        assert!(cx.debug_bounds("terminal-transport-error").is_some());
    }
}
