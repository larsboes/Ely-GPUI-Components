use std::rc::Rc;

use gpui::{
    AnyElement, App, ClickEvent, Div, ElementId, FontWeight, Hsla, InteractiveElement, IntoElement,
    MouseButton, ParentElement, Refineable, RenderOnce, Role, SharedString,
    StatefulInteractiveElement, StyleRefinement, Styled, Window, div, prelude::*, px,
};
use smallvec::SmallVec;

use crate::{
    data_display::{Badge, Tone},
    primitives::{FocusRing, Icon, IconName, Tooltip},
    theme::{ActiveTheme, IconSize, Radius, TextSize},
    typography::{Ellipsis, tabular},
};

type OnClick = Rc<dyn Fn(&ClickEvent, &mut Window, &mut App)>;

/// A widget's frame: its title with a line under it, actions at the end of the title's line, its body, and an optional footer. It fills the box it sits in, as a dashboard tile.
#[derive(IntoElement)]
pub struct DashboardCard {
    title: SharedString,
    subtitle: Option<SharedString>,
    actions: Vec<AnyElement>,
    body: Option<AnyElement>,
    footer: Option<AnyElement>,
}

impl DashboardCard {
    pub fn new(title: impl Into<SharedString>) -> Self {
        Self {
            title: title.into(),
            subtitle: None,
            actions: Vec::new(),
            body: None,
            footer: None,
        }
    }

    pub fn subtitle(mut self, text: impl Into<SharedString>) -> Self {
        self.subtitle = Some(text.into());
        self
    }

    pub fn action(mut self, action: impl IntoElement) -> Self {
        self.actions.push(action.into_any_element());
        self
    }

    pub fn body(mut self, body: impl IntoElement) -> Self {
        self.body = Some(body.into_any_element());
        self
    }

    pub fn footer(mut self, footer: impl IntoElement) -> Self {
        self.footer = Some(footer.into_any_element());
        self
    }
}

impl RenderOnce for DashboardCard {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme();
        div()
            .h_full()
            .flex()
            .flex_col()
            .gap_3()
            .p_4()
            .rounded(theme.radius(Radius::Lg))
            .border_1()
            .border_color(theme.colors.border)
            .bg(theme.colors.surface)
            .overflow_hidden()
            .child(
                div()
                    .flex()
                    .items_start()
                    .justify_between()
                    .gap_2()
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .flex()
                            .flex_col()
                            .child(
                                div()
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .child(Ellipsis::new(self.title)),
                            )
                            .children(self.subtitle.map(|text| {
                                div()
                                    .text_size(theme.text_size(TextSize::Xs))
                                    .text_color(theme.colors.fg_muted)
                                    .child(Ellipsis::new(text))
                            })),
                    )
                    .child(
                        div()
                            .flex_none()
                            .flex()
                            .items_center()
                            .gap_1()
                            .children(self.actions),
                    ),
            )
            .child(div().flex_1().min_h_0().children(self.body))
            .children(self.footer.map(|footer| {
                div()
                    .text_size(theme.text_size(TextSize::Xs))
                    .text_color(theme.colors.fg_subtle)
                    .child(footer)
            }))
    }
}

/// Compact metric tile with a tabular figure, optional detail caption, active filter badge, and click handler.
#[derive(IntoElement)]
pub struct MetricTile {
    id: ElementId,
    base: Div,
    label: SharedString,
    value: SharedString,
    detail: Option<SharedString>,
    tone: Tone,
    color: Option<Hsla>,
    active: bool,
    on_click: Option<OnClick>,
}

impl MetricTile {
    pub fn new(
        id: impl Into<ElementId>,
        label: impl Into<SharedString>,
        value: impl Into<SharedString>,
    ) -> Self {
        Self {
            id: id.into(),
            base: div().flex_1().min_w(px(120.)),
            label: label.into(),
            value: value.into(),
            detail: None,
            tone: Tone::Neutral,
            color: None,
            active: false,
            on_click: None,
        }
    }

    pub fn detail(mut self, text: impl Into<SharedString>) -> Self {
        self.detail = Some(text.into());
        self
    }

    pub fn tone(mut self, tone: impl Into<Tone>) -> Self {
        self.tone = tone.into();
        self
    }

    /// Explicit figure color override when the caller computes a domain color.
    pub fn color(mut self, color: impl Into<Hsla>) -> Self {
        self.color = Some(color.into());
        self
    }

    pub fn active(mut self, active: bool) -> Self {
        self.active = active;
        self
    }

    pub fn on_click(
        mut self,
        handler: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_click = Some(Rc::new(handler));
        self
    }
}

impl Styled for MetricTile {
    fn style(&mut self) -> &mut StyleRefinement {
        self.base.style()
    }
}

impl RenderOnce for MetricTile {
    fn render(mut self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme();
        let colors = &theme.colors;
        let value_color = self.color.unwrap_or_else(|| match self.tone {
            Tone::Neutral => colors.fg,
            Tone::Accent => colors.accent,
            Tone::Info => colors.info,
            Tone::Success => colors.success,
            Tone::Warning => colors.warning,
            Tone::Danger => colors.danger,
        });
        let interactive = self.on_click.is_some();
        let active = self.active;
        let (surface, border) = if active {
            (colors.accent.opacity(0.12), colors.accent)
        } else {
            (colors.surface, colors.border)
        };
        let (hover_bg, hover_border) = if active {
            (colors.accent.opacity(0.18), colors.accent)
        } else {
            (colors.hover, colors.accent.opacity(0.45))
        };
        let debug_id = self.id.clone();
        let mut tile = div()
            .id(self.id)
            .debug_selector(move || format!("metric-tile-{debug_id}"));
        tile.style().refine(self.base.style());
        tile.px_4()
            .py_3p5()
            .rounded(theme.radius(Radius::Lg))
            .bg(surface)
            .border_1()
            .border_color(border)
            .flex()
            .flex_col()
            .gap_1p5()
            .when(interactive, |tile| {
                tile.role(Role::Button)
                    .tab_index(0)
                    .focus_ring(cx)
                    .cursor_pointer()
                    .hover(move |s| s.bg(hover_bg).border_color(hover_border))
                    .on_mouse_down(MouseButton::Left, |_, window, _| window.prevent_default())
            })
            .when_some(self.on_click, |tile, on_click| {
                tile.on_click(move |event, window, cx| on_click(event, window, cx))
            })
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .gap_2()
                    .child(
                        div()
                            .text_size(theme.text_size(TextSize::Xs))
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(if active {
                                colors.accent
                            } else {
                                colors.fg_muted
                            })
                            .child(Ellipsis::new(self.label)),
                    )
                    .when(active, |row| {
                        row.child(Badge::new("Active").tone(Tone::Accent))
                    }),
            )
            .child(
                tabular(div())
                    .font_family(theme.mono_family.clone())
                    .text_size(theme.text_size(TextSize::Xxl))
                    .font_weight(FontWeight::BOLD)
                    .text_color(value_color)
                    .child(self.value),
            )
            .when_some(self.detail, |tile, detail| {
                tile.child(
                    div()
                        .text_size(theme.text_size(TextSize::Xs))
                        .text_color(if active { colors.fg } else { colors.fg_muted })
                        .child(Ellipsis::new(detail)),
                )
            })
    }
}

/// Horizontal wrapping strip of [`MetricTile`] or [`crate::data_display::KpiCard`] items.
#[derive(IntoElement, Default)]
pub struct MetricStrip {
    cards: SmallVec<[AnyElement; 6]>,
}

impl MetricStrip {
    pub fn new() -> Self {
        Self::default()
    }
}

impl ParentElement for MetricStrip {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.cards.extend(elements);
    }
}

impl RenderOnce for MetricStrip {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        div()
            .flex()
            .flex_wrap()
            .items_stretch()
            .gap_3()
            .children(self.cards)
    }
}

/// Circular icon-first quick action tile with a caption below and tooltip on hover.
#[derive(IntoElement)]
pub struct QuickActionTile {
    id: ElementId,
    icon: Option<IconName>,
    custom_icon: Option<AnyElement>,
    label: SharedString,
    tooltip: Option<SharedString>,
    active: bool,
    color: Option<Hsla>,
    on_click: Option<OnClick>,
}

impl QuickActionTile {
    pub fn new(
        id: impl Into<ElementId>,
        icon: IconName,
        label: impl Into<SharedString>,
    ) -> Self {
        Self {
            id: id.into(),
            icon: Some(icon),
            custom_icon: None,
            label: label.into(),
            tooltip: None,
            active: false,
            color: None,
            on_click: None,
        }
    }

    /// Creates a quick action tile with a custom icon element.
    pub fn with_element(
        id: impl Into<ElementId>,
        icon: impl IntoElement,
        label: impl Into<SharedString>,
    ) -> Self {
        Self {
            id: id.into(),
            icon: None,
            custom_icon: Some(icon.into_any_element()),
            label: label.into(),
            tooltip: None,
            active: false,
            color: None,
            on_click: None,
        }
    }

    pub fn tooltip(mut self, tooltip: impl Into<SharedString>) -> Self {
        self.tooltip = Some(tooltip.into());
        self
    }

    pub fn active(mut self, active: bool) -> Self {
        self.active = active;
        self
    }

    pub fn color(mut self, color: impl Into<Hsla>) -> Self {
        self.color = Some(color.into());
        self
    }

    pub fn on_click(
        mut self,
        handler: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_click = Some(Rc::new(handler));
        self
    }
}

impl RenderOnce for QuickActionTile {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme();
        let colors = &theme.colors;
        let accent = self.color.unwrap_or(colors.accent);
        let active = self.active;
        let (disc_bg, disc_border, fg) = if active {
            (accent.opacity(0.18), accent, accent)
        } else {
            (colors.hover, colors.border, colors.fg)
        };
        let hover_bg = if active {
            accent.opacity(0.26)
        } else {
            colors.accent.opacity(0.12)
        };
        let icon_el: AnyElement = match (self.icon, self.custom_icon) {
            (Some(icon), _) => Icon::new(icon).size(IconSize::Md).color(fg).into_any_element(),
            (None, Some(custom)) => div().text_color(fg).child(custom).into_any_element(),
            (None, None) => div().into_any_element(),
        };
        let debug_id = self.id.clone();
        div()
            .id(self.id)
            .debug_selector(move || format!("quick-action-{debug_id}"))
            .role(Role::Button)
            .aria_label(self.label.clone())
            .tab_index(0)
            .focus_ring(cx)
            .flex()
            .flex_col()
            .items_center()
            .gap_1p5()
            .min_w(px(64.))
            .cursor_pointer()
            .when_some(self.tooltip, |tile, tip| tile.tooltip(Tooltip::text(tip)))
            .on_mouse_down(MouseButton::Left, |_, window, _| window.prevent_default())
            .when_some(self.on_click, |tile, on_click| {
                tile.on_click(move |event, window, cx| on_click(event, window, cx))
            })
            .child(
                div()
                    .size(px(42.))
                    .rounded_full()
                    .flex()
                    .items_center()
                    .justify_center()
                    .bg(disc_bg)
                    .border_1()
                    .border_color(disc_border)
                    .text_color(fg)
                    .hover(move |s| s.bg(hover_bg).border_color(accent))
                    .child(icon_el),
            )
            .child(
                div()
                    .text_size(theme.text_size(TextSize::Xs))
                    .font_weight(if active {
                        FontWeight::SEMIBOLD
                    } else {
                        FontWeight::MEDIUM
                    })
                    .text_color(if active { accent } else { colors.fg_muted })
                    .child(self.label),
            )
    }
}
