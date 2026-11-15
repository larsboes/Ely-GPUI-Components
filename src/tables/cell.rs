use gpui::{
    AnyElement, App, ElementId, IntoElement, ParentElement, Rems, SharedString, Styled, div,
};

use crate::{
    buttons::ButtonVariant,
    data_display::{Avatar, Badge, Sparkline, Tone},
    motion::ProgressBar,
    theme::{ActiveTheme, AvatarSize, TextSize},
    typography::{Ellipsis, ExternalLink, format, tabular},
};

/// What a cell holds; its column says how numbers read.
#[derive(Clone, Debug, PartialEq)]
pub enum Cell {
    Empty,
    Text(SharedString),
    Number(f64),
    Tag(SharedString, Tone),
    /// A share from 0 to 1, drawn as a bar with its percent.
    Progress(f32),
    Spark(Vec<f32>),
    /// A person's name beside their initials.
    Person(SharedString),
    /// A label, and the address it opens.
    Link(SharedString, SharedString),
    /// A length of time in seconds, read in its two largest units; it sorts by length.
    Duration(u64),
    /// A number the app formats itself, such as money in its own locale: the text it shows, the value it sorts and adds by, and its tone. Neutral reads muted.
    Figure(SharedString, f64, Tone),
    /// A title over a muted line of detail; a filter and a sort read both.
    Lines(SharedString, SharedString),
    /// The keys of its column's actions this row offers. Any other cell in an actions column offers them all.
    Actions(Vec<SharedString>),
}

impl Cell {
    /// The words a filter looks through and a sort compares.
    pub(crate) fn words(&self) -> SharedString {
        match self {
            Self::Text(text)
            | Self::Tag(text, _)
            | Self::Person(text)
            | Self::Link(text, _)
            | Self::Figure(text, _, _) => text.clone(),
            Self::Lines(title, meta) => format!("{title} {meta}").into(),
            Self::Number(value) => value.to_string().into(),
            Self::Progress(share) => share.to_string().into(),
            Self::Duration(seconds) => {
                format::duration(*seconds, format::DurationStyle::Compact).into()
            }
            Self::Empty | Self::Spark(_) | Self::Actions(_) => SharedString::default(),
        }
    }

    /// The value a sort and a figure use, for cells that count.
    pub(crate) fn number(&self) -> Option<f64> {
        match self {
            Self::Number(value) => Some(*value),
            Self::Progress(share) => Some(f64::from(*share)),
            Self::Duration(seconds) => Some(*seconds as f64),
            Self::Figure(_, value, _) => Some(*value),
            _ => None,
        }
    }
}

impl From<&'static str> for Cell {
    fn from(text: &'static str) -> Self {
        Self::Text(text.into())
    }
}

impl From<String> for Cell {
    fn from(text: String) -> Self {
        Self::Text(text.into())
    }
}

impl From<f64> for Cell {
    fn from(value: f64) -> Self {
        Self::Number(value)
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Align {
    #[default]
    Start,
    End,
}

/// A figure the footer shows for a column.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Aggregate {
    Sum,
    Average,
    Count,
    Min,
    Max,
}

impl Aggregate {
    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::Sum => "Sum",
            Self::Average => "Avg",
            Self::Count => "Count",
            Self::Min => "Min",
            Self::Max => "Max",
        }
    }
}

/// A button an actions column draws on its rows; the table's `on_action` gets its key.
#[derive(Clone, Debug, PartialEq)]
pub struct RowAction {
    pub(crate) key: SharedString,
    pub(crate) label: SharedString,
    pub(crate) variant: ButtonVariant,
}

impl RowAction {
    pub fn new(key: impl Into<SharedString>, label: impl Into<SharedString>) -> Self {
        Self {
            key: key.into(),
            label: label.into(),
            variant: ButtonVariant::Ghost,
        }
    }

    /// Draws as the row's main action.
    pub fn primary(mut self) -> Self {
        self.variant = ButtonVariant::Primary;
        self
    }

    pub fn key(&self) -> &SharedString {
        &self.key
    }
}

/// A column: its key and title, its width, and how its cells read.
#[derive(Clone, Debug)]
pub struct Column {
    pub(crate) key: SharedString,
    pub(crate) title: SharedString,
    pub(crate) width: Option<Rems>,
    pub(crate) align: Align,
    pub(crate) sortable: bool,
    pub(crate) decimals: usize,
    pub(crate) prefix: Option<SharedString>,
    pub(crate) suffix: Option<SharedString>,
    pub(crate) aggregate: Option<Aggregate>,
    pub(crate) scale: bool,
    pub(crate) editable: bool,
    pub(crate) pinned: bool,
    pub(crate) muted: bool,
    pub(crate) actions: Vec<RowAction>,
}

impl Column {
    pub fn new(key: impl Into<SharedString>, title: impl Into<SharedString>) -> Self {
        Self {
            key: key.into(),
            title: title.into(),
            width: None,
            align: Align::Start,
            sortable: true,
            decimals: 0,
            prefix: None,
            suffix: None,
            aggregate: None,
            scale: false,
            editable: false,
            pinned: false,
            muted: false,
            actions: Vec::new(),
        }
    }

    /// A fixed width; columns without one share what is left.
    pub fn width(mut self, width: Rems) -> Self {
        self.width = Some(width);
        self
    }

    /// Sets its cells to the end, as numbers read.
    pub fn end(mut self) -> Self {
        self.align = Align::End;
        self
    }

    /// Its header does not sort.
    pub fn unsorted(mut self) -> Self {
        self.sortable = false;
        self
    }

    pub fn decimals(mut self, decimals: usize) -> Self {
        self.decimals = decimals;
        self
    }

    pub fn prefix(mut self, text: impl Into<SharedString>) -> Self {
        self.prefix = Some(text.into());
        self
    }

    pub fn suffix(mut self, text: impl Into<SharedString>) -> Self {
        self.suffix = Some(text.into());
        self
    }

    /// A figure for the footer, over the rows the filter keeps.
    pub fn aggregate(mut self, aggregate: Aggregate) -> Self {
        self.aggregate = Some(aggregate);
        self
    }

    /// Tints each number by where it sits in the column: red low, green high.
    pub fn scale(mut self) -> Self {
        self.scale = true;
        self
    }

    /// A double press on a cell edits its text; the table's `on_edit` gets it.
    pub fn editable(mut self) -> Self {
        self.editable = true;
        self
    }

    /// Holds still at the start while the other columns scroll sideways. Needs a width.
    pub fn pinned(mut self) -> Self {
        self.pinned = true;
        self
    }

    /// Its cells read in the muted text color, as a secondary column does.
    pub fn muted(mut self) -> Self {
        self.muted = true;
        self
    }

    /// Draws `actions` as buttons on each row, at the end; its header does not sort. A row's `Cell::Actions` names the ones it offers.
    pub fn actions(mut self, actions: impl IntoIterator<Item = RowAction>) -> Self {
        self.actions = actions.into_iter().collect();
        self.sortable = false;
        self.align = Align::End;
        self
    }

    /// The actions `cell` offers in this column, in the column's order.
    pub(crate) fn offered<'a>(&'a self, cell: &'a Cell) -> impl Iterator<Item = &'a RowAction> {
        self.actions.iter().filter(move |action| match cell {
            Cell::Actions(keys) => keys.contains(&action.key),
            _ => true,
        })
    }

    pub fn key(&self) -> &SharedString {
        &self.key
    }

    pub fn title(&self) -> &SharedString {
        &self.title
    }

    /// A number as this column reads it.
    pub(crate) fn reads(&self, value: f64) -> String {
        format!(
            "{}{}{}",
            self.prefix.as_ref().map_or("", |text| text.as_ref()),
            format::number(value, self.decimals, format::Separators::EN),
            self.suffix.as_ref().map_or("", |text| text.as_ref())
        )
    }
}

/// A cell drawn as its kind asks.
pub(crate) fn draw(cell: &Cell, column: &Column, id: ElementId, cx: &App) -> AnyElement {
    let colors = &cx.theme().colors;
    let drawn = match cell {
        Cell::Empty => div().into_any_element(),
        Cell::Text(text) => Ellipsis::new(text.clone()).into_any_element(),
        Cell::Number(value) => tabular(div())
            .child(column.reads(*value))
            .into_any_element(),
        Cell::Tag(text, tone) => Badge::new(text.clone()).tone(*tone).into_any_element(),
        Cell::Progress(share) => div()
            .flex()
            .items_center()
            .gap_2()
            .w_full()
            .child(
                div()
                    .flex_1()
                    .child(ProgressBar::new(id, share.clamp(0.0, 1.0))),
            )
            .child(
                tabular(div())
                    .text_color(colors.fg_muted)
                    .child(format::percent(f64::from(*share), 0, false)),
            )
            .into_any_element(),
        Cell::Spark(values) => Sparkline::new(values.iter().copied()).into_any_element(),
        Cell::Person(name) => div()
            .flex()
            .items_center()
            .gap_2()
            .min_w_0()
            .child(Avatar::new(id, name.clone()).size(AvatarSize::Xs))
            .child(Ellipsis::new(name.clone()))
            .into_any_element(),
        Cell::Duration(seconds) => tabular(div())
            .child(format::duration(*seconds, format::DurationStyle::Compact))
            .into_any_element(),
        Cell::Link(label, url) => {
            ExternalLink::new(id, label.clone(), url.clone()).into_any_element()
        }
        Cell::Figure(text, _, tone) => tabular(div())
            .text_color(figure_color(*tone, colors))
            .child(text.clone())
            .into_any_element(),
        Cell::Lines(title, meta) => div()
            .min_w_0()
            .flex()
            .flex_col()
            .child(Ellipsis::new(title.clone()))
            .child(
                div()
                    .text_size(cx.theme().text_size(TextSize::Xs))
                    .text_color(colors.fg_muted)
                    .child(Ellipsis::new(meta.clone())),
            )
            .into_any_element(),
        Cell::Actions(_) => div().into_any_element(),
    };
    if column.muted {
        div()
            .min_w_0()
            .text_color(colors.fg_muted)
            .child(drawn)
            .into_any_element()
    } else {
        drawn
    }
}

/// The text color a figure of `tone` takes.
fn figure_color(tone: Tone, colors: &crate::theme::Palette) -> gpui::Hsla {
    match tone {
        Tone::Neutral => colors.fg_muted,
        Tone::Accent => colors.accent,
        other => other.colors(colors).1,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tables::{Row, rules::sorted_by};

    #[test]
    fn a_figure_shows_its_text_and_sorts_by_its_value() {
        let figure = Cell::Figure("12,34 €".into(), 12.34, Tone::Danger);
        assert_eq!(figure.words(), "12,34 €");
        assert_eq!(figure.number(), Some(12.34));
        let rows: Vec<Row> = [
            ("a", "9,00 €", 9.0),
            ("b", "10,00 €", 10.0),
            ("c", "-1,00 €", -1.0),
        ]
        .into_iter()
        .map(|(key, text, value)| Row::new(key, [Cell::Figure(text.into(), value, Tone::Neutral)]))
        .collect();
        let order = sorted_by(&rows, vec![0, 1, 2], &[(0, true)]);
        assert_eq!(order, [2, 0, 1], "by value, not by its text");
    }

    #[test]
    fn lines_filter_and_sort_by_title_then_meta() {
        let cell = Cell::Lines("Chair".into(), "Oak · 2019".into());
        assert_eq!(cell.words(), "Chair Oak · 2019");
        assert_eq!(cell.number(), None);
        let rows: Vec<Row> = [
            ("a", "Table", "Pine"),
            ("b", "Chair", "Oak"),
            ("c", "Chair", "Ash"),
        ]
        .into_iter()
        .map(|(key, title, meta)| Row::new(key, [Cell::Lines(title.into(), meta.into())]))
        .collect();
        assert_eq!(sorted_by(&rows, vec![0, 1, 2], &[(0, true)]), [2, 1, 0]);
    }

    #[test]
    fn a_row_offers_the_actions_its_cell_names_or_all() {
        let column = Column::new("do", "").actions([
            RowAction::new("confirm", "Confirm").primary(),
            RowAction::new("reject", "Reject"),
        ]);
        assert!(!column.sortable, "actions do not sort");
        assert_eq!(column.align, Align::End);
        let keys = |cell: &Cell| -> Vec<String> {
            column
                .offered(cell)
                .map(|action| action.key.to_string())
                .collect()
        };
        assert_eq!(keys(&Cell::Empty), ["confirm", "reject"]);
        assert_eq!(keys(&Cell::Actions(vec!["reject".into()])), ["reject"]);
        assert!(keys(&Cell::Actions(Vec::new())).is_empty());
        assert_eq!(Cell::Actions(vec!["reject".into()]).words(), "");
    }

    #[test]
    fn a_column_is_plain_until_muted() {
        assert!(!Column::new("a", "A").muted);
        assert!(Column::new("a", "A").muted().muted);
    }
}
