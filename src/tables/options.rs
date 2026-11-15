use std::rc::Rc;

use gpui::{AnyElement, App, ElementId, SharedString, Window, div};

use super::{Column, FilterRule, Row, SortKey, table::DataTable};
use crate::theme::Density;

impl DataTable {
    pub fn new(id: impl Into<ElementId>, columns: impl IntoIterator<Item = Column>) -> Self {
        Self {
            id: id.into(),
            base: div(),
            columns: columns.into_iter().collect(),
            rows: Rc::default(),
            query: SharedString::default(),
            filters: Vec::new(),
            any: false,
            sorts: Vec::new(),
            hidden: Vec::new(),
            page_size: None,
            virtualized: false,
            selected: None,
            on_select: None,
            detail: None,
            group_by: None,
            on_edit: None,
            on_action: None,
            current: None,
            on_current: None,
            on_open: None,
            on_sort: None,
            density: Density::Standard,
            empty: None,
            scroll: None,
        }
    }

    /// A virtualized table's rows scroll by `handle`, so the app can scroll them to a row.
    pub fn track_scroll(mut self, handle: &gpui::UniformListScrollHandle) -> Self {
        self.scroll = Some(handle.clone());
        self
    }

    /// What the table says when no row shows, in place of "Nothing matches.".
    pub fn empty(mut self, text: impl Into<SharedString>) -> Self {
        self.empty = Some(text.into());
        self
    }

    /// The rows, owned or shared; a shared list is not copied.
    pub fn rows(mut self, rows: impl Into<Rc<Vec<Row>>>) -> Self {
        self.rows = rows.into();
        self
    }

    /// Keeps the rows holding this text in any cell.
    pub fn query(mut self, query: impl Into<SharedString>) -> Self {
        self.query = query.into();
        self
    }

    /// Rules from a `FilterBuilder`: rows pass them all, or any when `any`.
    pub fn filters(mut self, rules: impl IntoIterator<Item = FilterRule>, any: bool) -> Self {
        self.filters = rules.into_iter().collect();
        self.any = any;
        self
    }

    /// Sort keys from the owner, such as a `SortBuilder`'s; header presses change them until the owner sends new ones.
    pub fn sorts(mut self, keys: impl IntoIterator<Item = SortKey>) -> Self {
        self.sorts = keys
            .into_iter()
            .map(|key| (key.column, key.rising))
            .collect();
        self
    }

    /// Columns left out, such as a toolbar's menu hides.
    pub fn hidden(mut self, keys: impl IntoIterator<Item = impl Into<SharedString>>) -> Self {
        self.hidden = keys.into_iter().map(Into::into).collect();
        self
    }

    /// Splits the rows into pages of `size`.
    pub fn paged(mut self, size: usize) -> Self {
        assert!(size > 0, "a page holds at least a row");
        self.page_size = Some(size);
        self
    }

    /// Draws only the rows in view, for long tables. Give it a height.
    pub fn virtualized(mut self) -> Self {
        self.virtualized = true;
        self
    }

    /// Shows a box on each row; `keys` are the rows selected.
    pub fn selected(mut self, keys: impl IntoIterator<Item = impl Into<SharedString>>) -> Self {
        self.selected = Some(keys.into_iter().map(Into::into).collect());
        self
    }

    pub fn on_select(
        mut self,
        handler: impl Fn(&[SharedString], &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_select = Some(Rc::new(handler));
        self
    }

    /// A disclosure on each row opens what `detail` draws for it, under the row.
    pub fn detail(
        mut self,
        detail: impl Fn(&SharedString, &mut Window, &mut App) -> AnyElement + 'static,
    ) -> Self {
        self.detail = Some(Rc::new(detail));
        self
    }

    /// Gathers rows under their value in column `key`; each group folds.
    pub fn group_by(mut self, key: impl Into<SharedString>) -> Self {
        self.group_by = Some(key.into());
        self
    }

    /// Gets a row's key, a column's key and the new text, after a double press on an editable cell.
    pub fn on_edit(
        mut self,
        handler: impl Fn(&SharedString, &SharedString, &SharedString, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_edit = Some(Rc::new(handler));
        self
    }

    /// Gets a row's key and an action's key, after a press on a button in an actions column.
    pub fn on_action(
        mut self,
        handler: impl Fn(&SharedString, &SharedString, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_action = Some(Rc::new(handler));
        self
    }

    /// The row the keys move from; up, down, j and k move it, and Enter opens it. Tab reaches the table.
    pub fn current(mut self, key: Option<impl Into<SharedString>>) -> Self {
        self.current = key.map(Into::into);
        self
    }

    /// Gets the key of the row that became current, after a key or a press moved it.
    pub fn on_current(
        mut self,
        handler: impl Fn(&SharedString, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_current = Some(Rc::new(handler));
        self
    }

    /// Gets a row's key after Enter on the current row or a press on a row. The row becomes current first.
    pub fn on_open(
        mut self,
        handler: impl Fn(&SharedString, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_open = Some(Rc::new(handler));
        self
    }

    /// Gets the sort keys after a press on a header changed them.
    pub fn on_sort(
        mut self,
        handler: impl Fn(&[SortKey], &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_sort = Some(Rc::new(handler));
        self
    }

    pub fn density(mut self, density: Density) -> Self {
        self.density = density;
        self
    }
}
