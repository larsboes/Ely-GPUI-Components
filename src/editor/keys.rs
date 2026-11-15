use gpui::{App, Context, InteractiveElement, Window, actions};

use super::{buffer::Buffer, cursor::Motion, state::CodeEditor};

actions!(
    ely_editor,
    [
        Left,
        Right,
        Up,
        Down,
        SelectLeft,
        SelectRight,
        SelectUp,
        SelectDown,
        WordLeft,
        WordRight,
        SelectWordLeft,
        SelectWordRight,
        LineStart,
        LineEnd,
        SelectLineStart,
        SelectLineEnd,
        DocumentStart,
        DocumentEnd,
        SelectDocumentStart,
        SelectDocumentEnd,
        PageUp,
        PageDown,
        Backspace,
        Delete,
        DeleteWordLeft,
        DeleteLineLeft,
        Newline,
        Indent,
        Outdent,
        Undo,
        Redo,
        SelectAll,
        Copy,
        Cut,
        Paste,
        AddCursorAbove,
        AddCursorBelow,
        SelectNext,
        SingleCursor,
        ToggleComment,
        Fold,
        Unfold,
    ]
);

pub(crate) const CONTEXT: &str = "ElyEditor";

/// Binds the editor's keys. `init` calls it.
pub(crate) fn bind_keys(cx: &mut App) {
    let word = if cfg!(target_os = "macos") {
        "alt"
    } else {
        "ctrl"
    };
    let context = Some(CONTEXT);
    let mut bindings = vec![
        crate::bind(cx, "left", Left, context),
        crate::bind(cx, "right", Right, context),
        crate::bind(cx, "up", Up, context),
        crate::bind(cx, "down", Down, context),
        crate::bind(cx, "shift-left", SelectLeft, context),
        crate::bind(cx, "shift-right", SelectRight, context),
        crate::bind(cx, "shift-up", SelectUp, context),
        crate::bind(cx, "shift-down", SelectDown, context),
        crate::bind(cx, &format!("{word}-left"), WordLeft, context),
        crate::bind(cx, &format!("{word}-right"), WordRight, context),
        crate::bind(cx, &format!("{word}-shift-left"), SelectWordLeft, context),
        crate::bind(cx, &format!("{word}-shift-right"), SelectWordRight, context),
        crate::bind(cx, "home", LineStart, context),
        crate::bind(cx, "end", LineEnd, context),
        crate::bind(cx, "shift-home", SelectLineStart, context),
        crate::bind(cx, "shift-end", SelectLineEnd, context),
        crate::bind(cx, "pageup", PageUp, context),
        crate::bind(cx, "pagedown", PageDown, context),
        crate::bind(cx, "backspace", Backspace, context),
        crate::bind(cx, "shift-backspace", Backspace, context),
        crate::bind(cx, "delete", Delete, context),
        crate::bind(cx, &format!("{word}-backspace"), DeleteWordLeft, context),
        crate::bind(cx, "enter", Newline, context),
        crate::bind(cx, "tab", Indent, context),
        crate::bind(cx, "shift-tab", Outdent, context),
        crate::bind(cx, "secondary-z", Undo, context),
        crate::bind(cx, "secondary-shift-z", Redo, context),
        crate::bind(cx, "secondary-a", SelectAll, context),
        crate::bind(cx, "secondary-c", Copy, context),
        crate::bind(cx, "secondary-x", Cut, context),
        crate::bind(cx, "secondary-d", SelectNext, context),
        crate::bind(cx, "escape", SingleCursor, context),
        crate::bind(cx, "secondary-/", ToggleComment, context),
        crate::bind(cx, "secondary-alt-[", Fold, context),
        crate::bind(cx, "secondary-alt-]", Unfold, context),
    ];
    // The browser's paste event reaches the input handler.
    if !cfg!(target_family = "wasm") {
        bindings.push(crate::bind(cx, "secondary-v", Paste, context));
    }
    if cfg!(target_os = "macos") {
        bindings.extend([
            crate::bind(cx, "cmd-left", LineStart, context),
            crate::bind(cx, "cmd-right", LineEnd, context),
            crate::bind(cx, "cmd-shift-left", SelectLineStart, context),
            crate::bind(cx, "cmd-shift-right", SelectLineEnd, context),
            crate::bind(cx, "cmd-up", DocumentStart, context),
            crate::bind(cx, "cmd-down", DocumentEnd, context),
            crate::bind(cx, "cmd-shift-up", SelectDocumentStart, context),
            crate::bind(cx, "cmd-shift-down", SelectDocumentEnd, context),
            crate::bind(cx, "cmd-backspace", DeleteLineLeft, context),
            crate::bind(cx, "cmd-alt-up", AddCursorAbove, context),
            crate::bind(cx, "cmd-alt-down", AddCursorBelow, context),
        ]);
    } else {
        bindings.extend([
            crate::bind(cx, "ctrl-home", DocumentStart, context),
            crate::bind(cx, "ctrl-end", DocumentEnd, context),
            crate::bind(cx, "ctrl-shift-home", SelectDocumentStart, context),
            crate::bind(cx, "ctrl-shift-end", SelectDocumentEnd, context),
            crate::bind(cx, "ctrl-alt-up", AddCursorAbove, context),
            crate::bind(cx, "ctrl-alt-down", AddCursorBelow, context),
            crate::bind(cx, "ctrl-y", Redo, context),
        ]);
    }
    cx.bind_keys(bindings);
}

/// Rows a page key moves.
const PAGE: isize = 20;

fn line_left(buffer: &Buffer, at: usize) -> std::ops::Range<usize> {
    buffer.line_range(buffer.line_of(at)).start..at
}

/// Wires every action to the editor that renders `root`.
pub(crate) fn listen<E: InteractiveElement>(root: E, cx: &mut Context<CodeEditor>) -> E {
    macro_rules! on {
        ($root:expr, $action:ty, $body:expr) => {
            $root.on_action(cx.listener(
                move |editor: &mut CodeEditor,
                      _: &$action,
                      window: &mut Window,
                      cx: &mut Context<CodeEditor>| {
                    let run: fn(&mut CodeEditor, &mut Window, &mut Context<CodeEditor>) = $body;
                    run(editor, window, cx)
                },
            ))
        };
    }
    let root = on!(root, Left, |e, _, cx| e.motion(Motion::Left, false, cx));
    let root = on!(root, Right, |e, _, cx| e.motion(Motion::Right, false, cx));
    let root = on!(root, Up, |e, _, cx| e.motion(Motion::Up, false, cx));
    let root = on!(root, Down, |e, _, cx| e.motion(Motion::Down, false, cx));
    let root = on!(root, SelectLeft, |e, _, cx| e.motion(
        Motion::Left,
        true,
        cx
    ));
    let root = on!(root, SelectRight, |e, _, cx| e.motion(
        Motion::Right,
        true,
        cx
    ));
    let root = on!(root, SelectUp, |e, _, cx| e.motion(Motion::Up, true, cx));
    let root = on!(root, SelectDown, |e, _, cx| e.motion(
        Motion::Down,
        true,
        cx
    ));
    let root = on!(root, WordLeft, |e, _, cx| e.motion(
        Motion::WordLeft,
        false,
        cx
    ));
    let root = on!(root, WordRight, |e, _, cx| e.motion(
        Motion::WordRight,
        false,
        cx
    ));
    let root = on!(root, SelectWordLeft, |e, _, cx| e.motion(
        Motion::WordLeft,
        true,
        cx
    ));
    let root = on!(root, SelectWordRight, |e, _, cx| e.motion(
        Motion::WordRight,
        true,
        cx
    ));
    let root = on!(root, LineStart, |e, _, cx| e.motion(
        Motion::LineStart,
        false,
        cx
    ));
    let root = on!(root, LineEnd, |e, _, cx| e.motion(
        Motion::LineEnd,
        false,
        cx
    ));
    let root = on!(root, SelectLineStart, |e, _, cx| e.motion(
        Motion::LineStart,
        true,
        cx
    ));
    let root = on!(root, SelectLineEnd, |e, _, cx| e.motion(
        Motion::LineEnd,
        true,
        cx
    ));
    let root = on!(root, DocumentStart, |e, _, cx| e.motion(
        Motion::Start,
        false,
        cx
    ));
    let root = on!(root, DocumentEnd, |e, _, cx| e.motion(
        Motion::End,
        false,
        cx
    ));
    let root = on!(root, SelectDocumentStart, |e, _, cx| e.motion(
        Motion::Start,
        true,
        cx
    ));
    let root = on!(root, SelectDocumentEnd, |e, _, cx| e.motion(
        Motion::End,
        true,
        cx
    ));
    let root = on!(root, PageUp, |e, _, cx| e.motion(
        Motion::Page(-PAGE),
        false,
        cx
    ));
    let root = on!(root, PageDown, |e, _, cx| e.motion(
        Motion::Page(PAGE),
        false,
        cx
    ));
    let root = on!(root, Backspace, |e, _, cx| {
        e.delete_by(|buffer, at| buffer.previous(at)..at, cx)
    });
    let root = on!(root, Delete, |e, _, cx| e
        .delete_by(|buffer, at| at..buffer.next(at), cx));
    let root = on!(root, DeleteWordLeft, |e, _, cx| {
        e.delete_by(|buffer, at| buffer.word_start(at)..at, cx)
    });
    let root = on!(root, DeleteLineLeft, |e, _, cx| e.delete_by(line_left, cx));
    let root = on!(root, Newline, |e, window, cx| {
        if e.focus.is_focused(window) {
            e.newline(cx)
        } else {
            cx.propagate()
        }
    });
    let root = on!(root, Indent, |e, _, cx| e.indent(cx));
    let root = on!(root, Outdent, |e, _, cx| e.outdent(cx));
    let root = on!(root, Undo, |e, _, cx| e.undo(cx));
    let root = on!(root, Redo, |e, _, cx| e.redo(cx));
    let root = on!(root, SelectAll, |e, _, cx| e.select_all(cx));
    let root = on!(root, Copy, |e, _, cx| e.copy(cx));
    let root = on!(root, Cut, |e, _, cx| e.cut(cx));
    let root = on!(root, Paste, |e, _, cx| {
        if let Some(text) = cx.read_from_clipboard().and_then(|item| item.text()) {
            e.paste(&text, cx)
        }
    });
    let root = on!(root, AddCursorAbove, |e, _, cx| e.add_cursor(false, cx));
    let root = on!(root, AddCursorBelow, |e, _, cx| e.add_cursor(true, cx));
    let root = on!(root, SelectNext, |e, _, cx| e.select_next(cx));
    let root = on!(root, SingleCursor, |e, window, cx| {
        if e.marks.chat.is_some() {
            e.set_chat(None, window, cx)
        } else {
            e.single_cursor(cx)
        }
    });
    let root = on!(root, ToggleComment, |e, _, cx| e.toggle_comment(cx));
    let root = on!(root, Fold, |e, _, cx| {
        let line = e.buffer.line_of(e.primary().head);
        let header = super::syntax::folds(&e.buffer)
            .into_iter()
            .filter(|(header, end)| *header <= line && line <= *end)
            .map(|(header, _)| header)
            .next_back();
        if let Some(header) = header.filter(|header| !e.folded.contains(header)) {
            e.toggle_fold(header, cx);
        }
    });
    on!(root, Unfold, |e, _, cx| {
        let line = e.buffer.line_of(e.primary().head);
        if e.folded.contains(&line) {
            e.toggle_fold(line, cx);
        }
    })
}
