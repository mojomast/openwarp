use warp_opencode::pty::{CellColor, PtyState};

fn line(state: &PtyState, row: usize) -> String {
    (0..state.grid().cols())
        .map(|col| state.grid().cell(row, col).unwrap().ch)
        .collect::<String>()
        .trim_end()
        .to_string()
}

#[test]
fn plain_text_renders_into_grid() {
    let mut state = PtyState::new(3, 10);
    state.feed("hello");

    assert_eq!(line(&state, 0), "hello");
    assert_eq!(state.grid().cursor(), (0, 5));
}

#[test]
fn sgr_bold_color_and_reset() {
    let mut state = PtyState::new(2, 10);
    state.feed("\x1b[1;31mR\x1b[0mN");

    let red = state.grid().cell(0, 0).unwrap();
    assert_eq!(red.ch, 'R');
    assert!(red.bold);
    assert_eq!(red.fg, CellColor::Indexed(1));

    let normal = state.grid().cell(0, 1).unwrap();
    assert_eq!(normal.ch, 'N');
    assert!(!normal.bold);
    assert_eq!(normal.fg, CellColor::Default);
}

#[test]
fn carriage_return_and_linefeed() {
    let mut state = PtyState::new(3, 10);
    state.feed("abc\rZ\nnext");

    assert_eq!(line(&state, 0), "Zbc");
    // LF does not imply CR in terminal emulation, so the next text starts at col 1.
    assert_eq!(line(&state, 1), " next");
}

#[test]
fn csi_2j_clears_display() {
    let mut state = PtyState::new(2, 10);
    state.feed("hello\x1b[2J");

    assert_eq!(line(&state, 0), "");
    assert_eq!(line(&state, 1), "");
}

#[test]
fn cup_moves_cursor() {
    let mut state = PtyState::new(4, 8);
    state.feed("\x1b[2;3HX");

    assert_eq!(state.grid().cell(1, 2).unwrap().ch, 'X');
    assert_eq!(state.grid().cursor(), (1, 3));
}

#[test]
fn scrollback_collects_scrolled_lines() {
    let mut state = PtyState::new(3, 8);
    state.feed("one\r\ntwo\r\nthree\r\nfour");

    assert_eq!(state.grid().scrollback_len(), 1);
    assert_eq!(line(&state, 0), "two");
    assert_eq!(line(&state, 2), "four");
}

#[test]
fn resize_preserves_visible_cells() {
    let mut state = PtyState::new(2, 5);
    state.feed("abc\r\ndef");
    state.resize(3, 8);

    assert_eq!(state.grid().rows(), 3);
    assert_eq!(state.grid().cols(), 8);
    assert_eq!(line(&state, 0), "abc");
    assert_eq!(line(&state, 1), "def");
}
