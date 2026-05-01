use std::{borrow::Cow, ops::Range, time::Duration};

use ratatui::{
    layout::Layout,
    macros::span,
    style::{Color, Style},
    widgets::Widget,
};

use crate::application::theme::PATINA;

// TODO: Currently scrolling is stateless, so it doesn't differentiate between
// the user scrolling up or down. this means the selection stays at the bottom
// even if the user is scrolling.
//
// To resolve this, we need additional book keeping on the "last scroll
// direction", and using that to switch the scrolling algo.

/// Given a number of items, max number of items which can be shown, and a possibly selected index:
/// returns the range of items that must be rendered
pub fn selected_scroll(items: usize, max_items: usize, selected: Option<usize>) -> Range<usize> {
    let Some(selected) = selected else {
        let min = max_items.min(items);
        return 0..min;
    };

    if items <= max_items || selected < max_items {
        return 0..items.min(max_items);
    }

    let end = selected + 1;
    let start = end - max_items;
    start..end
}

pub type CowStr = Cow<'static, str>;

#[inline]
pub fn strength_bars(s: f32) -> &'static str {
    if s >= 75.0 {
        "▮▮▮▮"
    } else if s >= 55.0 {
        "▮▮▮▯"
    } else if s >= 35.0 {
        "▮▮▯▯"
    } else if s >= 15.0 {
        "▮▯▯▯"
    } else {
        "▯▯▯▯"
    }
}

#[inline]
pub fn humanize_duration(d: Duration) -> String {
    timeago::Formatter::new().convert(d)
}

#[inline]
pub fn strength_color(s: f32) -> Color {
    if s >= 55.0 {
        PATINA.live
    } else if s >= 35.0 {
        PATINA.warn
    } else {
        PATINA.danger
    }
}

const BRAILLE_BITS: [[u8; 2]; 4] = [[0x01, 0x08], [0x02, 0x10], [0x04, 0x20], [0x40, 0x80]];

pub struct BrailleSparkline<'a> {
    data: &'a [usize],
    style: Style,
    max: Option<usize>,
}

impl<'a> BrailleSparkline<'a> {
    pub fn new(data: &'a [usize]) -> Self {
        Self {
            data,
            style: Style::new(),
            max: None,
        }
    }

    pub fn style(mut self, style: Style) -> Self {
        self.style = style;
        self
    }

    pub fn max(mut self, max: usize) -> Self {
        self.max = Some(max);
        self
    }
}

impl Widget for BrailleSparkline<'_> {
    fn render(self, area: ratatui::prelude::Rect, buf: &mut ratatui::prelude::Buffer)
    where
        Self: Sized,
    {
        buf.set_style(area, self.style);

        if area.width == 0 || area.height == 0 || self.data.is_empty() {
            return;
        }

        let n = area.width as usize;
        let m = area.height as usize;
        let sub_cols = 2 * n;
        let sub_rows = 4 * m;

        let slice = if self.data.len() > sub_cols {
            &self.data[self.data.len() - sub_cols..]
        } else {
            self.data
        };

        let max = self
            .max
            .unwrap_or_else(|| slice.iter().copied().max().unwrap_or(0));
        if max == 0 {
            return;
        }

        let mut masks = vec![0u8; n * m];

        let max_f = max as f64;
        let top_bin = (sub_rows - 1) as f64;

        for (sx, &v) in slice.iter().enumerate() {
            let raw = (v as f64 / max_f * top_bin).round();
            let bin = if raw.is_nan() || raw < 0.0 {
                0
            } else if raw > top_bin {
                sub_rows - 1
            } else {
                raw as usize
            };
            let sy = (sub_rows - 1) - bin;

            let cell_col = sx / 2;
            let cell_row = sy / 4;
            let local_col = sx % 2;
            let local_row = sy % 4;

            masks[cell_row * n + cell_col] |= BRAILLE_BITS[local_row][local_col];
        }

        for cell_row in 0..m {
            for cell_col in 0..n {
                let mask = masks[cell_row * n + cell_col];
                if mask == 0 {
                    continue;
                }
                let glyph = char::from_u32(0x2800 | mask as u32).unwrap();
                let x = area.x + cell_col as u16;
                let y = area.y + cell_row as u16;
                if let Some(cell) = buf.cell_mut((x, y)) {
                    cell.set_char(glyph).set_style(self.style);
                }
            }
        }
    }
}

pub struct WidgetList<T> {
    children: Vec<T>,
    layout: Layout,
}

impl<T: Widget> WidgetList<T> {
    pub fn new(layout: Layout, children: impl IntoIterator<Item = T>) -> Self {
        Self {
            layout,
            children: children.into_iter().collect(),
        }
    }
}

impl<T: Widget> Widget for WidgetList<T> {
    fn render(self, area: ratatui::prelude::Rect, buf: &mut ratatui::prelude::Buffer)
    where
        Self: Sized,
    {
        let areas = self.layout.split(area);

        self.children
            .into_iter()
            .zip(areas.iter().cloned())
            .for_each(|(w, a)| {
                w.render(a, buf);
            });
    }
}

pub enum Either<L, R> {
    Left(L),
    Right(R),
}

impl<L: Widget, R: Widget> Widget for Either<L, R> {
    fn render(self, area: ratatui::prelude::Rect, buf: &mut ratatui::prelude::Buffer)
    where
        Self: Sized,
    {
        match self {
            Either::Left(l) => l.render(area, buf),
            Either::Right(r) => r.render(area, buf),
        }
    }
}

pub struct Separator {
    pub char: char,
    pub style: Style,
}

impl Separator {
    pub const fn new(c: char) -> Self {
        Self {
            char: c,
            style: Style::new(),
        }
    }

    pub const fn styled(self, style: Style) -> Self {
        Self {
            char: self.char,
            style,
        }
    }
}

impl Widget for Separator {
    fn render(self, area: ratatui::prelude::Rect, buf: &mut ratatui::prelude::Buffer)
    where
        Self: Sized,
    {
        let text: String = std::iter::repeat_n(self.char, area.width as usize).collect();

        let text = span![self.style; text];

        text.render(area, buf);
    }
}

#[cfg(test)]
mod tests {
    use super::selected_scroll;

    #[test]
    fn none_both_zero() {
        assert_eq!(selected_scroll(0, 0, None), 0..0);
    }

    #[test]
    fn none_zero_items() {
        assert_eq!(selected_scroll(0, 5, None), 0..0);
    }

    #[test]
    fn none_zero_max() {
        assert_eq!(selected_scroll(5, 0, None), 0..0);
    }

    #[test]
    fn none_items_below_max() {
        assert_eq!(selected_scroll(3, 5, None), 0..3);
    }

    #[test]
    fn none_items_equal_max() {
        assert_eq!(selected_scroll(5, 5, None), 0..5);
    }

    #[test]
    fn none_items_above_max() {
        assert_eq!(selected_scroll(10, 3, None), 0..3);
    }

    #[test]
    fn some_fits_first() {
        assert_eq!(selected_scroll(3, 5, Some(0)), 0..3);
    }

    #[test]
    fn some_fits_last() {
        assert_eq!(selected_scroll(3, 5, Some(2)), 0..3);
    }

    #[test]
    fn some_exact_first() {
        assert_eq!(selected_scroll(5, 5, Some(0)), 0..5);
    }

    #[test]
    fn some_exact_last() {
        assert_eq!(selected_scroll(5, 5, Some(4)), 0..5);
    }

    #[test]
    fn some_overflow_first_window_start() {
        assert_eq!(selected_scroll(10, 3, Some(0)), 0..3);
    }

    #[test]
    fn some_overflow_first_window_end() {
        assert_eq!(selected_scroll(10, 3, Some(2)), 0..3);
    }

    #[test]
    fn some_overflow_max_one_first() {
        assert_eq!(selected_scroll(10, 1, Some(0)), 0..1);
    }

    #[test]
    fn some_scroll_just_past_first_window() {
        assert_eq!(selected_scroll(10, 3, Some(3)), 1..4);
    }

    #[test]
    fn some_scroll_middle() {
        assert_eq!(selected_scroll(10, 3, Some(5)), 3..6);
    }

    #[test]
    fn some_scroll_last() {
        assert_eq!(selected_scroll(10, 3, Some(9)), 7..10);
    }

    #[test]
    fn some_scroll_max_one_middle() {
        assert_eq!(selected_scroll(10, 1, Some(5)), 5..6);
    }

    #[test]
    fn some_scroll_max_one_last() {
        assert_eq!(selected_scroll(10, 1, Some(9)), 9..10);
    }

    #[test]
    fn some_scroll_large() {
        assert_eq!(selected_scroll(100, 10, Some(50)), 41..51);
    }

    #[test]
    fn edge_zero_items_with_selection() {
        assert_eq!(selected_scroll(0, 5, Some(0)), 0..0);
    }

    #[test]
    fn edge_all_zero_with_selection() {
        assert_eq!(selected_scroll(0, 0, Some(0)), 0..0);
    }

    #[test]
    fn edge_zero_max_no_selection() {
        assert_eq!(selected_scroll(5, 0, None), 0..0);
    }

    #[test]
    fn edge_zero_max_with_selection() {
        assert_eq!(selected_scroll(5, 0, Some(2)), 3..3);
    }

    #[test]
    fn edge_selected_past_end_fits() {
        assert_eq!(selected_scroll(5, 10, Some(7)), 0..5);
    }

    #[test]
    fn edge_selected_past_end_scroll() {
        assert_eq!(selected_scroll(5, 3, Some(5)), 3..6);
    }

    #[test]
    fn edge_minimal_scrolled() {
        assert_eq!(selected_scroll(1, 1, Some(0)), 0..1);
    }
}
