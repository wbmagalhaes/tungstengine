#[cfg(not(target_arch = "wasm32"))]
thread_local! {
    static BOARD: std::cell::RefCell<Option<arboard::Clipboard>> =
        std::cell::RefCell::new(arboard::Clipboard::new().ok());
}

#[cfg(target_arch = "wasm32")]
thread_local! {
    static BOARD: std::cell::RefCell<String> = const { std::cell::RefCell::new(String::new()) };
}

#[cfg(not(target_arch = "wasm32"))]
pub fn get() -> Option<String> {
    BOARD.with_borrow_mut(|b| b.as_mut()?.get_text().ok())
}

#[cfg(not(target_arch = "wasm32"))]
pub fn set(text: &str) {
    BOARD.with_borrow_mut(|b| {
        if let Some(b) = b.as_mut() {
            let _ = b.set_text(text.to_string());
        }
    });
}

#[cfg(target_arch = "wasm32")]
pub fn get() -> Option<String> {
    BOARD.with_borrow(|b| Some(b.clone()))
}

#[cfg(target_arch = "wasm32")]
pub fn set(text: &str) {
    BOARD.with_borrow_mut(|b| *b = text.to_string());
}
