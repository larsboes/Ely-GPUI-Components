use std::rc::Rc;

use gpui::{
    AnyElement, App, ElementId, FontWeight, InteractiveElement, IntoElement, MouseButton,
    ParentElement, RenderOnce, SharedString, StatefulInteractiveElement, Styled, Window, div,
    prelude::*,
};
use smallvec::SmallVec;

use crate::{
    forms::{OnFlag, Run},
    layout::Collapsible,
    primitives::{FocusRing, Icon, IconName, Tooltip},
    theme::{ActiveTheme, ControlSize, IconSize, Radius, TextSize},
};

/// One place in a side navigation. A folded sidebar shows the icon alone and names it on hover.
#[derive(IntoElement)]
pub struct NavItem {
    id: ElementId,
    label: SharedString,
    icon: IconName,
    count: Option<SharedString>,
    active: bool,
    folded: bool,
    on_click: Option<Run>,
}

impl NavItem {
    pub fn new(id: impl Into<ElementId>, icon: IconName, label: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            icon,
            count: None,
            active: false,
            folded: false,
            on_click: None,
        }
    }

    /// Quiet text at the end, such as an unread count.
    pub fn count(mut self, count: impl Into<SharedString>) -> Self {
        self.count = Some(count.into());
        self
    }

    /// Marks the place you are.
    pub fn active(mut self, active: bool) -> Self {
        self.active = active;
        self
    }

    /// The icon alone, for a sidebar folded to a rail.
    pub fn folded(mut self, folded: bool) -> Self {
        self.folded = folded;
        self
    }

    pub fn on_click(mut self, handler: impl Fn(&mut Window, &mut App) + 'static) -> Self {
        self.on_click = Some(Rc::new(handler));
        self
    }
}

impl RenderOnce for NavItem {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme();
        let colors = &theme.colors;
        let fg = if self.active {
            colors.fg
        } else {
            colors.fg_muted
        };
        let (id, label) = (self.id.clone(), self.label.clone());
        let item = div()
            .id(self.id)
            .relative()
            .flex()
            .items_center()
            .gap_2()
            .h(theme.control_height(ControlSize::Md))
            .rounded(theme.radius(Radius::Md))
            .border_1()
            .border_color(gpui::transparent_black())
            .text_size(theme.text_size(TextSize::Sm))
            .text_color(fg)
            .cursor_pointer()
            .tab_index(0)
            .focus_ring(cx)
            .on_mouse_down(MouseButton::Left, |_, window, _| window.prevent_default())
            .map(|item| {
                if self.active {
                    item.bg(colors.hover).font_weight(FontWeight::MEDIUM).child(
                        div()
                            .absolute()
                            .left_0()
                            .top_1p5()
                            .bottom_1p5()
                            .w(theme.tab_indicator())
                            .rounded_full()
                            .bg(colors.accent),
                    )
                } else {
                    item.hover(|style| style.bg(colors.hover).text_color(colors.fg))
                }
            })
            .when_some(self.on_click, |item, click| {
                item.on_click(move |_, window, cx| {
                    log::info!("nav item {id:?}: open");
                    click(window, cx)
                })
            })
            .child(Icon::new(self.icon).size(IconSize::Sm).color(fg));
        if self.folded {
            return item
                .justify_center()
                .w(theme.control_height(ControlSize::Md))
                .tooltip(Tooltip::text(label));
        }
        item.px_2()
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .overflow_hidden()
                    .whitespace_nowrap()
                    .text_ellipsis()
                    .child(label),
            )
            .when_some(self.count, |item, count| {
                item.child(div().text_color(colors.fg_subtle).child(count))
            })
    }
}

/// A titled run of nav items. The title folds it; a folded sidebar drops the title.
#[derive(IntoElement)]
pub struct NavGroup {
    id: ElementId,
    title: SharedString,
    open: bool,
    folded: bool,
    items: SmallVec<[AnyElement; 8]>,
    on_toggle: Option<OnFlag>,
}

impl NavGroup {
    pub fn new(id: impl Into<ElementId>, title: impl Into<SharedString>, open: bool) -> Self {
        Self {
            id: id.into(),
            title: title.into(),
            open,
            folded: false,
            items: SmallVec::new(),
            on_toggle: None,
        }
    }

    /// Items only, for a sidebar folded to a rail.
    pub fn folded(mut self, folded: bool) -> Self {
        self.folded = folded;
        self
    }

    /// Runs with the state the title asks for.
    pub fn on_toggle(mut self, handler: impl Fn(bool, &mut Window, &mut App) + 'static) -> Self {
        self.on_toggle = Some(Rc::new(handler));
        self
    }
}

impl ParentElement for NavGroup {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.items.extend(elements);
    }
}

impl RenderOnce for NavGroup {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let items = div().flex().flex_col().gap_0p5().children(self.items);
        if self.folded {
            return div().id(self.id).child(items);
        }
        let theme = cx.theme();
        let colors = &theme.colors;
        let (open, id) = (self.open, self.id.clone());
        let toggle = self.on_toggle;
        div()
            .id(self.id.clone())
            .flex()
            .flex_col()
            .gap_0p5()
            .child(
                div()
                    .id((self.id.clone(), "title"))
                    .flex()
                    .items_center()
                    .gap_1()
                    .h(theme.control_height(ControlSize::Sm))
                    .px_2()
                    .rounded(theme.radius(Radius::Md))
                    .border_1()
                    .border_color(gpui::transparent_black())
                    .text_size(theme.text_size(TextSize::Xs))
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(colors.fg_subtle)
                    .cursor_pointer()
                    .tab_index(0)
                    .focus_ring(cx)
                    .hover(|style| style.text_color(colors.fg_muted))
                    .on_mouse_down(MouseButton::Left, |_, window, _| window.prevent_default())
                    .on_click(move |_, window, cx| {
                        log::info!("nav group {id:?}: open {}", !open);
                        if let Some(toggle) = &toggle {
                            toggle(!open, window, cx);
                        }
                    })
                    .child(
                        Icon::new(if open {
                            IconName::ChevronDown
                        } else {
                            IconName::ChevronRight
                        })
                        .size(IconSize::Xs)
                        .color(colors.fg_subtle),
                    )
                    .child(self.title),
            )
            .child(Collapsible::new((self.id, "items"), open).child(items))
    }
}

/// An item in a [`BottomTabBar`]: key, label, icon text or glyph, and optional badge count.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BottomTabItem {
    pub id: SharedString,
    pub label: SharedString,
    pub icon: SharedString,
    pub badge: Option<usize>,
}

impl BottomTabItem {
    pub fn new(
        id: impl Into<SharedString>,
        icon: impl Into<SharedString>,
        label: impl Into<SharedString>,
    ) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            icon: icon.into(),
            badge: None,
        }
    }

    pub fn badge(mut self, badge: Option<usize>) -> Self {
        self.badge = badge;
        self
    }
}

type OnSelectTab = Rc<dyn Fn(&SharedString, &mut Window, &mut App)>;

/// Touch-friendly bottom tab bar respecting 44pt+ minimum touch targets.
#[derive(IntoElement)]
pub struct BottomTabBar {
    id: ElementId,
    items: Vec<BottomTabItem>,
    active_id: SharedString,
    on_select: Option<OnSelectTab>,
}

impl BottomTabBar {
    pub fn new(
        id: impl Into<ElementId>,
        items: impl IntoIterator<Item = BottomTabItem>,
        active_id: impl Into<SharedString>,
    ) -> Self {
        Self {
            id: id.into(),
            items: items.into_iter().collect(),
            active_id: active_id.into(),
            on_select: None,
        }
    }

    pub fn on_select(
        mut self,
        handler: impl Fn(&SharedString, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_select = Some(Rc::new(handler));
        self
    }
}

impl RenderOnce for BottomTabBar {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme();
        let colors = &theme.colors;
        let on_select = self.on_select;
        let active_id = self.active_id;
        div()
            .id(self.id)
            .flex()
            .items_center()
            .justify_around()
            .h(gpui::px(56.))
            .bg(colors.surface)
            .border_t_1()
            .border_color(colors.border)
            .px_2()
            .children(self.items.into_iter().enumerate().map(|(idx, item)| {
                let is_active = item.id == active_id;
                let select = on_select.clone();
                let item_id = item.id.clone();
                let fg = if is_active {
                    colors.accent
                } else {
                    colors.fg_muted
                };
                div()
                    .id(SharedString::from(format!("nav-tab-{idx}")))
                    .flex()
                    .flex_col()
                    .items_center()
                    .justify_center()
                    .min_w(gpui::px(64.))
                    .min_h(gpui::px(44.))
                    .cursor_pointer()
                    .gap_1()
                    .on_click(move |_, window, cx| {
                        if let Some(select) = &select {
                            select(&item_id, window, cx);
                        }
                    })
                    .child(
                        div()
                            .relative()
                            .flex()
                            .items_center()
                            .justify_center()
                            .child(
                                div()
                                    .text_size(theme.text_size(TextSize::Sm))
                                    .font_weight(if is_active {
                                        FontWeight::BOLD
                                    } else {
                                        FontWeight::MEDIUM
                                    })
                                    .text_color(fg)
                                    .child(item.icon),
                            )
                            .when_some(item.badge.filter(|c| *c > 0), |el, count| {
                                el.child(
                                    div()
                                        .absolute()
                                        .top(gpui::px(-4.))
                                        .right(gpui::px(-10.))
                                        .min_w(gpui::px(16.))
                                        .h(gpui::px(16.))
                                        .px(gpui::px(4.))
                                        .rounded_full()
                                        .bg(colors.warning)
                                        .flex()
                                        .items_center()
                                        .justify_center()
                                        .text_color(colors.on_accent)
                                        .text_size(theme.text_size(TextSize::Xs))
                                        .font_weight(FontWeight::BOLD)
                                        .child(format!("{count}")),
                                )
                            }),
                    )
                    .child(
                        div()
                            .text_size(theme.text_size(TextSize::Xs))
                            .font_weight(if is_active {
                                FontWeight::SEMIBOLD
                            } else {
                                FontWeight::NORMAL
                            })
                            .text_color(fg)
                            .child(item.label),
                    )
            }))
    }
}

/// Top navigation bar for mobile detail views and compact headers.
#[derive(IntoElement)]
pub struct NavHeader {
    title: SharedString,
    subtitle: Option<SharedString>,
    on_back: Option<Run>,
    trailing: Option<AnyElement>,
}

impl NavHeader {
    pub fn new(title: impl Into<SharedString>) -> Self {
        Self {
            title: title.into(),
            subtitle: None,
            on_back: None,
            trailing: None,
        }
    }

    pub fn subtitle(mut self, subtitle: impl Into<SharedString>) -> Self {
        self.subtitle = Some(subtitle.into());
        self
    }

    pub fn on_back(mut self, handler: impl Fn(&mut Window, &mut App) + 'static) -> Self {
        self.on_back = Some(Rc::new(handler));
        self
    }

    pub fn trailing(mut self, trailing: impl IntoElement) -> Self {
        self.trailing = Some(trailing.into_any_element());
        self
    }
}

impl RenderOnce for NavHeader {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme();
        let colors = &theme.colors;
        let left = match self.on_back {
            Some(on_back) => div()
                .id("nav-back")
                .flex()
                .items_center()
                .gap_1()
                .cursor_pointer()
                .min_h(gpui::px(44.))
                .text_size(theme.text_size(TextSize::Sm))
                .font_weight(FontWeight::MEDIUM)
                .text_color(colors.accent)
                .child("← Back")
                .on_click(move |_, window, cx| on_back(window, cx))
                .into_any_element(),
            None => div().w(gpui::px(60.)).into_any_element(),
        };
        let center = div()
            .flex()
            .flex_col()
            .items_center()
            .child(
                div()
                    .text_size(theme.text_size(TextSize::Sm))
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(colors.fg)
                    .child(self.title),
            )
            .when_some(self.subtitle, |el, sub| {
                el.child(
                    div()
                        .text_size(theme.text_size(TextSize::Xs))
                        .text_color(colors.fg_muted)
                        .child(sub),
                )
            });
        let right = match self.trailing {
            Some(actions) => div()
                .flex()
                .items_center()
                .gap_2()
                .child(actions)
                .into_any_element(),
            None => div().w(gpui::px(60.)).into_any_element(),
        };
        div()
            .flex()
            .items_center()
            .justify_between()
            .px_4()
            .py_3()
            .bg(colors.surface)
            .border_b_1()
            .border_color(colors.border)
            .child(left)
            .child(center)
            .child(right)
    }
}

#[cfg(test)]
mod tests {
    use super::{BottomTabBar, BottomTabItem, NavHeader};

    #[test]
    fn bottom_tab_item_and_header_builders_preserve_fields() {
        let item = BottomTabItem::new("inbox", "tray", "Inbox").badge(Some(4));
        assert_eq!(item.id.as_ref(), "inbox");
        assert_eq!(item.icon.as_ref(), "tray");
        assert_eq!(item.label.as_ref(), "Inbox");
        assert_eq!(item.badge, Some(4));

        let bar = BottomTabBar::new("mobile-tabs", [item], "inbox");
        assert_eq!(bar.items.len(), 1);
        assert_eq!(bar.active_id.as_ref(), "inbox");

        let header = NavHeader::new("Trip").subtitle("Tokyo");
        assert_eq!(header.title.as_ref(), "Trip");
        assert_eq!(header.subtitle.as_deref(), Some("Tokyo"));
    }
}
