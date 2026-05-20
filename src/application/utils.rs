use std::{borrow::Cow, ops::Range, time::Duration};

use ratatui::{
    layout::Layout,
    macros::span,
    style::{Color, Style},
    widgets::Widget,
};

use crate::application::theme::PATINA;

/// Represents the direction of the last scroll action
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScrollDirection {
    Up,
    Down,
}

/// Tracks scrolling state to differentiate between upward and downward scrolling
#[derive(Debug, Clone, Copy)]
pub struct ScrollState {
    /// The last selected index before the current selection
    pub last_selected: Option<usize>,
    /// The direction of the last scroll action
    pub last_direction: Option<ScrollDirection>,
}

impl ScrollState {
    /// Create a new, uninitialized scroll state
    pub fn new() -> Self {
        Self {
            last_selected: None,
            last_direction: None,
        }
    }

    pub fn advance(self, selected: Option<usize>) -> Self {
        let direction = match (self.last_selected, selected) {
            (Some(last), Some(current)) if current > last => ScrollDirection::Down,
            (Some(last), Some(current)) if current < last => ScrollDirection::Up,
            _ => self.last_direction.unwrap_or(ScrollDirection::Down),
        };

        Self {
            last_selected: selected,
            last_direction: Some(direction),
        }
    }
}

impl Default for ScrollState {
    fn default() -> Self {
        Self::new()
    }
}

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

/// Performs scrolling with awareness of scroll direction
///
/// This function tracks whether the user is scrolling up or down and applies
/// the appropriate scrolling algorithm:
/// - When scrolling **up**: keeps the selected item near the top of the visible range
/// - When scrolling **down**: keeps the selected item near the bottom of the visible range
///
/// # Returns
/// A tuple of (scroll_range, updated_state)
pub fn selected_scroll_with_direction(
    items: usize,
    max_items: usize,
    selected: Option<usize>,
    state: ScrollState,
) -> (Range<usize>, ScrollState) {
    let new_state = state.advance(selected);
    let direction = new_state.last_direction.unwrap_or(ScrollDirection::Down);

    // Apply direction-specific scrolling algorithm
    let range = match direction {
        ScrollDirection::Up => selected_scroll_up(items, max_items, selected),
        ScrollDirection::Down => selected_scroll_down(items, max_items, selected),
    };

    (range, new_state)
}

fn selected_scroll_with_anchor(
    items: usize,
    max_items: usize,
    selected: Option<usize>,
    anchor_from_top: usize,
) -> Range<usize> {
    let Some(selected) = selected else {
        return 0..max_items.min(items);
    };

    if items <= max_items {
        return 0..items;
    }

    let max_start = items - max_items;
    let start = selected.saturating_sub(anchor_from_top).min(max_start);
    start..(start + max_items)
}

/// Scrolls with selection kept near the top when moving up
fn selected_scroll_up(items: usize, max_items: usize, selected: Option<usize>) -> Range<usize> {
    // Keep selection at the top row while moving up.
    let anchor_from_top = 0;
    selected_scroll_with_anchor(items, max_items, selected, anchor_from_top)
}

/// Scrolls with selection kept near the bottom when moving down (original behavior)
fn selected_scroll_down(items: usize, max_items: usize, selected: Option<usize>) -> Range<usize> {
    // Keep selection near the bottom area (last row).
    let anchor_from_top = max_items.saturating_sub(1);
    selected_scroll_with_anchor(items, max_items, selected, anchor_from_top)
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
    use super::{selected_scroll, selected_scroll_with_direction, ScrollDirection, ScrollState};

    // ===== Original selected_scroll tests =====

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

    // ===== Stateful scroll with direction tests =====

    #[test]
    fn stateful_initial_state_defaults_down() {
        let state = ScrollState::new();
        let (range, new_state) = selected_scroll_with_direction(10, 3, Some(0), state);
        
        // First call with no prior state should behave like normal scroll (down)
        assert_eq!(range, 0..3);
        assert_eq!(new_state.last_selected, Some(0));
        assert_eq!(new_state.last_direction, Some(ScrollDirection::Down));
    }

    #[test]
    fn scroll_state_advance_tracks_direction_changes() {
        let state = ScrollState::new().advance(Some(4));
        assert_eq!(state.last_selected, Some(4));
        assert_eq!(state.last_direction, Some(ScrollDirection::Down));

        let state = state.advance(Some(2));
        assert_eq!(state.last_selected, Some(2));
        assert_eq!(state.last_direction, Some(ScrollDirection::Up));
    }

    #[test]
    fn stateful_scrolling_down() {
        let state = ScrollState {
            last_selected: Some(3),
            last_direction: Some(ScrollDirection::Down),
        };
        
        // Moving from index 3 to 5 (downward)
        let (range, new_state) = selected_scroll_with_direction(10, 3, Some(5), state);
        
        // Should keep selection near bottom
        assert_eq!(range, 3..6);
        assert_eq!(new_state.last_selected, Some(5));
        assert_eq!(new_state.last_direction, Some(ScrollDirection::Down));
    }

    #[test]
    fn stateful_scrolling_up() {
        let state = ScrollState {
            last_selected: Some(5),
            last_direction: Some(ScrollDirection::Down),
        };
        
        // Moving from index 5 to 2 (upward)
        let (range, new_state) = selected_scroll_with_direction(10, 3, Some(2), state);
        
        // Should keep selection near top of visible range
        assert_eq!(range, 2..5);
        assert_eq!(new_state.last_selected, Some(2));
        assert_eq!(new_state.last_direction, Some(ScrollDirection::Up));
    }

    #[test]
    fn stateful_rapid_down_scrolling() {
        let items = 20;
        let max_items = 5;
        
        let mut state = ScrollState::new();
        
        // Simulate rapid downward scrolling
        for selected in &[0, 2, 4, 6, 8, 10] {
            let (range, new_state) = selected_scroll_with_direction(items, max_items, Some(*selected), state);
            assert_eq!(new_state.last_direction, Some(ScrollDirection::Down));
            // Selection should be near the end of visible range
            assert!(range.contains(selected));
            state = new_state;
        }
        
        // Final state should have tracked last position
        assert_eq!(state.last_selected, Some(10));
    }

    #[test]
    fn stateful_rapid_up_scrolling() {
        let items = 20;
        let max_items = 5;
        
        let mut state = ScrollState {
            last_selected: Some(10),
            last_direction: Some(ScrollDirection::Down),
        };
        
        // Simulate rapid upward scrolling
        for selected in &[8, 6, 4, 2, 0] {
            let (range, new_state) = selected_scroll_with_direction(items, max_items, Some(*selected), state);
            assert_eq!(new_state.last_direction, Some(ScrollDirection::Up));
            // Selection should be within visible range
            assert!(range.contains(selected));
            state = new_state;
        }
        
        // Final state should have tracked last position
        assert_eq!(state.last_selected, Some(0));
    }

    #[test]
    fn stateful_alternating_direction_changes() {
        let items = 15;
        let max_items = 3;
        
        let state = ScrollState::new();
        
        // Start at index 2
        let (_range1, state) = selected_scroll_with_direction(items, max_items, Some(2), state);
        assert_eq!(_range1, 0..3);
        
        // Move down to index 8
        let (_range2, state) = selected_scroll_with_direction(items, max_items, Some(8), state);
        assert_eq!(state.last_direction, Some(ScrollDirection::Down));
        
        // Move back up to index 4
        let (range3, state) = selected_scroll_with_direction(items, max_items, Some(4), state);
        assert_eq!(state.last_direction, Some(ScrollDirection::Up));
        assert!(range3.contains(&4));
        
        // Move down again to index 10
        let (_range4, state) = selected_scroll_with_direction(items, max_items, Some(10), state);
        assert_eq!(state.last_direction, Some(ScrollDirection::Down));
        assert_eq!(state.last_selected, Some(10));
    }

    #[test]
    fn stateful_scroll_with_none_selection() {
        let state = ScrollState {
            last_selected: Some(5),
            last_direction: Some(ScrollDirection::Down),
        };
        
        // When selection is None, should show from start
        let (range, new_state) = selected_scroll_with_direction(10, 3, None, state);
        assert_eq!(range, 0..3);
        assert_eq!(new_state.last_selected, None);
    }

    #[test]
    fn stateful_scroll_edge_case_all_items_visible() {
        let state = ScrollState::new();
        
        // When all items fit in view, direction shouldn't matter
        let (range, _) = selected_scroll_with_direction(5, 10, Some(3), state);
        assert_eq!(range, 0..5);
    }

    #[test]
    fn stateful_scroll_maintains_boundaries() {
        let items = 10;
        let max_items = 3;
        
        let state = ScrollState {
            last_selected: Some(7),
            last_direction: Some(ScrollDirection::Down),
        };
        
        // Moving to the very last item
        let (range, _) = selected_scroll_with_direction(items, max_items, Some(9), state);
        
        // Should not exceed item boundaries
        assert!(range.end <= items);
        assert_eq!(range.len(), max_items);
        assert!(range.contains(&9));
    }
}
