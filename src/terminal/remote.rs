use std::rc::Rc;

use alacritty_terminal::vte::ansi::Processor;
use gpui::{Context, SharedString};

use super::{pty, view::Terminal};

type Input = Rc<dyn Fn(&[u8]) -> anyhow::Result<()>>;
type Resize = Rc<dyn Fn(usize, usize) -> anyhow::Result<()>>;

/// The host's ordered input and resize transport; no local process is started.
pub struct RemoteTransport {
    input: Input,
    resize: Resize,
}

impl RemoteTransport {
    /// Callbacks enqueue input in order and report transport failures without blocking the UI.
    pub fn new(
        input: impl Fn(&[u8]) -> anyhow::Result<()> + 'static,
        resize: impl Fn(usize, usize) -> anyhow::Result<()> + 'static,
    ) -> Self {
        Self {
            input: Rc::new(input),
            resize: Rc::new(resize),
        }
    }

    pub(super) fn input(&self, bytes: &[u8]) -> anyhow::Result<()> {
        (self.input)(bytes)
    }

    pub(super) fn resize(&self, columns: usize, lines: usize) -> anyhow::Result<()> {
        (self.resize)(columns, lines)
    }
}

pub(super) struct Remote {
    pub parser: Processor,
    pub transport: Option<RemoteTransport>,
}

impl Terminal {
    /// A live grid fed by the host; the host retains offsets, reconnect and process ownership.
    pub fn remote(
        columns: usize,
        lines: usize,
        transport: RemoteTransport,
        cx: &mut Context<Self>,
    ) -> Self {
        let size = (columns.max(2), lines.max(1));
        let session = pty::detached(alacritty_terminal::term::test::TermSize::new(
            size.0, size.1,
        ));
        let mut terminal = Self::new(session, size, "Remote terminal".into(), cx);
        terminal.remote = Some(Remote {
            parser: Processor::new(),
            transport: Some(transport),
        });
        terminal
    }

    /// Feeds one ordered chunk without resetting the escape-sequence parser.
    ///
    /// # Errors
    /// Returns an error for a local, replayed or closed terminal.
    pub fn feed_remote(&mut self, bytes: &[u8], cx: &mut Context<Self>) -> anyhow::Result<()> {
        let remote = self
            .remote
            .as_mut()
            .filter(|remote| remote.transport.is_some())
            .ok_or_else(|| anyhow::anyhow!("terminal: no open remote transport"))?;
        remote.parser.advance(&mut *self.term.lock(), bytes);
        cx.notify();
        Ok(())
    }

    /// Stops remote input and freezes the grid; the host closes its stream and process separately.
    pub fn finish_remote(&mut self, cx: &mut Context<Self>) {
        if let Some(remote) = &mut self.remote {
            remote.transport = None;
            cx.notify();
        }
    }

    /// Starts a fresh remote stream after the host detects a gap in retained output.
    ///
    /// # Errors
    /// Returns an error unless the remote transport is open.
    pub fn reset_remote(&mut self, cx: &mut Context<Self>) -> anyhow::Result<()> {
        use alacritty_terminal::vte::ansi::{ClearMode, Handler};
        let remote = self
            .remote
            .as_mut()
            .filter(|remote| remote.transport.is_some())
            .ok_or_else(|| anyhow::anyhow!("terminal: no open remote transport"))?;
        remote.parser = Processor::new();
        let mut term = self.term.lock();
        term.reset_state();
        term.clear_screen(ClearMode::Saved);
        self.search = super::search::Search::default();
        self.hovered = None;
        self.selecting = false;
        self.rest = gpui::Pixels::ZERO;
        self.shown = Rc::default();
        cx.notify();
        Ok(())
    }

    /// The most recent input or resize transport failure.
    pub fn transport_error(&self) -> Option<&SharedString> {
        self.transport_error.as_ref()
    }
}

#[cfg(all(test, feature = "test-support"))]
mod tests {
    use super::*;
    use alacritty_terminal::{
        index::{Column, Line},
        vte::ansi::{Color, NamedColor},
    };
    use gpui::{AppContext, ClipboardItem, TestAppContext};
    use std::{cell::RefCell, rc::Rc};

    #[gpui::test]
    fn chunks_keep_parser_state_and_reset_drops_an_incomplete_sequence(cx: &mut TestAppContext) {
        let view = cx
            .new(|cx| Terminal::remote(20, 3, RemoteTransport::new(|_| Ok(()), |_, _| Ok(())), cx));
        view.update(cx, |terminal, cx| {
            assert!(terminal.pty.is_none());
            terminal.feed_remote(b"\x1b[3", cx).unwrap();
            terminal.feed_remote(b"1mx", cx).unwrap();
            let term = terminal.term.lock();
            assert_eq!(term.grid()[Line(0)][Column(0)].c, 'x');
            assert_eq!(
                term.grid()[Line(0)][Column(0)].fg,
                Color::Named(NamedColor::Red)
            );
            drop(term);
            terminal.select_all(cx);
            terminal
                .find("x", crate::editor::FindOptions::default(), cx)
                .unwrap();
            terminal.selecting = true;
            terminal.feed_remote(b"\x1b[", cx).unwrap();
            terminal.reset_remote(cx).unwrap();
            assert!(terminal.selected_text().is_none());
            assert_eq!(terminal.matches(), (0, None));
            assert!(!terminal.selecting);
            terminal.feed_remote(b"fresh", cx).unwrap();
            assert_eq!(terminal.term.lock().grid()[Line(0)][Column(0)].c, 'f');
        });
    }

    #[gpui::test]
    fn input_resize_and_shutdown_stay_with_the_transport(cx: &mut TestAppContext) {
        let input = Rc::new(RefCell::new(Vec::new()));
        let sizes = Rc::new(RefCell::new(Vec::new()));
        let (typed, sized) = (input.clone(), sizes.clone());
        let view = cx.new(|cx| {
            Terminal::remote(
                20,
                3,
                RemoteTransport::new(
                    move |bytes| {
                        typed.borrow_mut().push(bytes.to_vec());
                        Ok(())
                    },
                    move |columns, lines| {
                        sized.borrow_mut().push((columns, lines));
                        Ok(())
                    },
                ),
                cx,
            )
        });
        view.update(cx, |terminal, cx| {
            terminal.write(b"a".to_vec(), cx);
            terminal.write(b"b".to_vec(), cx);
            terminal.resize(40, 5);
            terminal.resize(40, 5);
            terminal.feed_remote(b"kept", cx).unwrap();
            terminal.clear(cx);
            assert_eq!(terminal.term.lock().grid()[Line(0)][Column(0)].c, 'k');
            terminal.finish_remote(cx);
            terminal.write(b"closed".to_vec(), cx);
            terminal.resize(60, 7);
            assert!(terminal.feed_remote(b"late", cx).is_err());
        });
        assert_eq!(
            *input.borrow(),
            [b"a".to_vec(), b"b".to_vec(), b"\x0c".to_vec()]
        );
        assert_eq!(*sizes.borrow(), [(40, 5)]);
    }

    #[gpui::test]
    fn host_controls_select_copy_and_paste_without_a_local_process(cx: &mut TestAppContext) {
        let input = Rc::new(RefCell::new(Vec::new()));
        let typed = input.clone();
        let view = cx.new(|cx| {
            Terminal::remote(
                20,
                3,
                RemoteTransport::new(
                    move |bytes| {
                        typed.borrow_mut().extend_from_slice(bytes);
                        Ok(())
                    },
                    |_, _| Ok(()),
                ),
                cx,
            )
        });
        view.update(cx, |terminal, cx| {
            terminal.feed_remote(b"hello\x1b[?2004h", cx).unwrap();
            terminal.select_all(cx);
            assert!(terminal.selected_text().unwrap().starts_with("hello"));
            terminal.paste_text("text\n", cx);
            assert!(terminal.selected_text().is_none());
            assert_eq!(*input.borrow(), b"\x1b[200~text\r\x1b[201~");
            assert_eq!(terminal.dimensions(), (20, 3));
        });
    }

    #[gpui::test]
    fn failed_transport_is_reported_and_remote_output_cannot_access_clipboard(
        cx: &mut TestAppContext,
    ) {
        cx.update(|cx| cx.write_to_clipboard(ClipboardItem::new_string("owner".into())));
        let view = cx.new(|cx| {
            Terminal::remote(
                20,
                3,
                RemoteTransport::new(
                    |_| anyhow::bail!("disconnected"),
                    |_, _| anyhow::bail!("resize refused"),
                ),
                cx,
            )
        });
        view.update(cx, |terminal, cx| {
            terminal.write(b"a".to_vec(), cx);
            assert!(terminal.transport_error().unwrap().contains("disconnected"));
            terminal.resize(40, 5);
            assert!(
                terminal
                    .transport_error()
                    .unwrap()
                    .contains("resize refused")
            );
            terminal
                .feed_remote(b"\x1b]52;c;cmVtb3Rl\x07\x1b]52;c;?\x07", cx)
                .unwrap();
        });
        cx.run_until_parked();
        cx.update(|cx| {
            assert_eq!(
                cx.read_from_clipboard()
                    .and_then(|item| item.text())
                    .as_deref(),
                Some("owner")
            )
        });
    }
}
