#[derive(Default, Clone)]
pub struct EditInput {
    pub typed: String,
    pub backspace: bool,
    pub delete: bool,
    pub enter: bool,
    pub escape: bool,
    pub left: bool,
    pub right: bool,
    pub up: bool,
    pub down: bool,
    pub home: bool,
    pub end: bool,
    pub shift: bool,
    pub ctrl: bool,
    pub select_all: bool,
    pub copy: bool,
    pub cut: bool,
    pub paste: Option<String>,
}

#[derive(Default, Clone)]
pub struct CursorInput {
    pub pos: (f32, f32),
    pub clicked: bool,
    pub pressed: bool,
}

pub fn prev_word(chars: &[char], caret: usize) -> usize {
    let mut i = caret.min(chars.len());
    while i > 0 && chars[i - 1].is_whitespace() {
        i -= 1;
    }
    while i > 0 && !chars[i - 1].is_whitespace() {
        i -= 1;
    }
    i
}

pub fn next_word(chars: &[char], caret: usize) -> usize {
    let len = chars.len();
    let mut i = caret.min(len);
    while i < len && !chars[i].is_whitespace() {
        i += 1;
    }
    while i < len && chars[i].is_whitespace() {
        i += 1;
    }
    i
}
