use std::{
    collections::{HashMap, HashSet},
    rc::Rc,
};

use gpui::{
    App, Div, ElementId, IntoElement, Pixels, ScrollHandle, SharedString, StyleRefinement, Styled,
    UniformListScrollHandle, Window,
};

use super::{
    Cell, Column, FilterRule, SortKey,
    body::{Detail, OnAction, OnEdit},
};
use crate::theme::Density;

/// One row: its key, which a selection names it by, and a cell for each column.
#[derive(Clone, Debug, PartialEq)]
pub struct Row {
    pub(crate) key: SharedString,
    pub(crate) cells: Vec<Cell>,
}

impl Row {
    pub fn new(key: impl Into<SharedString>, cells: impl IntoIterator<Item = Cell>) -> Self {
        Self {
            key: key.into(),
            cells: cells.into_iter().collect(),
        }
    }
}

/// A table's own state: the page, a Shift range's start, the scrolls down and sideways, the header last scrolled into view, column widths, edges and order, opened rows, folded groups, and the cell being edited.
#[derive(Default)]
pub(crate) struct View {
    pub page: usize,
    pub anchor: usize,
    pub scroll: UniformListScrollHandle,
    pub sideways: ScrollHandle,
    pub revealed: Option<SharedString>,
    pub widths: HashMap<SharedString, Pixels>,
    pub edges: HashMap<SharedString, Pixels>,
    pub narrowest: Pixels,
    pub order: Vec<SharedString>,
    pub open: HashSet<SharedString>,
    pub folded: HashSet<SharedString>,
    pub editing: Option<(SharedString, SharedString)>,
}

pub(super) type OnSelect = Rc<dyn Fn(&[SharedString], &mut Window, &mut App)>;
pub(super) type OnRow = Rc<dyn Fn(&SharedString, &mut Window, &mut App)>;
pub(super) type OnSort = Rc<dyn Fn(&[SortKey], &mut Window, &mut App)>;

/// Rows under headers. A press on a header sorts, rising then falling then off; Shift adds it to the sort. A header drags to move its column and its edge drags to resize it. A query and filter rules keep rows; pages split long results, or a long table draws only the rows in view. Rows can gather into folding groups, open a detail, or edit a cell in place. With a selection, a box leads each row. Footers show figures; tinted columns show where each number sits.
#[derive(IntoElement)]
pub struct DataTable {
    pub(super) id: ElementId,
    pub(super) base: Div,
    pub(super) label: Option<SharedString>,
    pub(super) columns: Vec<Column>,
    pub(super) rows: Rc<Vec<Row>>,
    pub(super) query: SharedString,
    pub(super) filters: Vec<FilterRule>,
    pub(super) any: bool,
    pub(super) sorts: Vec<(SharedString, bool)>,
    pub(super) hidden: Vec<SharedString>,
    pub(super) page_size: Option<usize>,
    pub(super) virtualized: bool,
    pub(super) selected: Option<Vec<SharedString>>,
    pub(super) on_select: Option<OnSelect>,
    pub(super) detail: Option<Detail>,
    pub(super) group_by: Option<SharedString>,
    pub(super) on_edit: Option<OnEdit>,
    pub(super) on_action: Option<OnAction>,
    pub(super) current: Option<SharedString>,
    pub(super) on_current: Option<OnRow>,
    pub(super) on_open: Option<OnRow>,
    pub(super) on_sort: Option<OnSort>,
    pub(super) density: Density,
    pub(super) empty: Option<SharedString>,
    pub(super) scroll: Option<UniformListScrollHandle>,
}

impl Styled for DataTable {
    fn style(&mut self) -> &mut StyleRefinement {
        self.base.style()
    }
}
