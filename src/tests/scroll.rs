use crate::application::utils::{
    ScrollDirection, ScrollState, selected_scroll, selected_scroll_with_anchor,
    selected_scroll_with_direction,
};

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
    assert_eq!(new_state.window_start, 0);
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
        window_start: 1,
    };

    // Moving from index 3 to 5 (downward)
    let (range, new_state) = selected_scroll_with_direction(10, 3, Some(5), state);

    // Should keep selection near bottom
    assert_eq!(range, 3..6);
    assert_eq!(new_state.last_selected, Some(5));
    assert_eq!(new_state.last_direction, Some(ScrollDirection::Down));
    assert_eq!(new_state.window_start, 3);
}

#[test]
fn stateful_scrolling_up() {
    let state = ScrollState {
        last_selected: Some(5),
        last_direction: Some(ScrollDirection::Down),
        window_start: 3,
    };

    // Moving from index 5 to 2 (upward)
    let (range, new_state) = selected_scroll_with_direction(10, 3, Some(2), state);

    // Should move minimally while keeping selection visible
    assert_eq!(range, 2..5);
    assert_eq!(new_state.last_selected, Some(2));
    assert_eq!(new_state.last_direction, Some(ScrollDirection::Up));
    assert_eq!(new_state.window_start, 2);
}

#[test]
fn stateful_rapid_down_scrolling() {
    let items = 20;
    let max_items = 5;

    let mut state = ScrollState::new();

    // Simulate rapid downward scrolling
    for selected in &[0, 2, 4, 6, 8, 10] {
        let (range, new_state) =
            selected_scroll_with_direction(items, max_items, Some(*selected), state);
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
        window_start: 6,
    };

    // Simulate rapid upward scrolling
    for selected in &[8, 6, 4, 2, 0] {
        let (range, new_state) =
            selected_scroll_with_direction(items, max_items, Some(*selected), state);
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
    let (range1, state) = selected_scroll_with_direction(items, max_items, Some(2), state);
    assert_eq!(range1, 0..3);

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
        window_start: 4,
    };

    // When selection is None, should show from start
    let (range, new_state) = selected_scroll_with_direction(10, 3, None, state);
    assert_eq!(range, 0..3);
    assert_eq!(new_state.last_selected, None);
    assert_eq!(new_state.window_start, 0);
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
        window_start: 6,
    };

    // Moving to the very last item
    let (range, _) = selected_scroll_with_direction(items, max_items, Some(9), state);

    // Should not exceed item boundaries
    assert!(range.end <= items);
    assert_eq!(range.len(), max_items);
    assert!(range.contains(&9));
}

// ===== Anchor helper tests =====

#[test]
fn anchor_keeps_window_when_selection_inside() {
    let (range, start) = selected_scroll_with_anchor(20, 5, Some(7), 5);
    assert_eq!(range, 5..10);
    assert_eq!(start, 5);
}

#[test]
fn anchor_moves_window_up_when_selection_is_above() {
    let (range, start) = selected_scroll_with_anchor(20, 5, Some(3), 8);
    assert_eq!(range, 3..8);
    assert_eq!(start, 3);
}

#[test]
fn anchor_moves_window_down_when_selection_is_below() {
    let (range, start) = selected_scroll_with_anchor(20, 5, Some(12), 5);
    assert_eq!(range, 8..13);
    assert_eq!(start, 8);
}

#[test]
fn anchor_clamps_window_at_end() {
    let (range, start) = selected_scroll_with_anchor(10, 4, Some(99), 6);
    assert_eq!(range, 6..10);
    assert_eq!(start, 6);
}