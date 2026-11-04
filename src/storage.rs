#[cfg(not(target_arch = "wasm32"))]
pub fn read(key: &str) -> Option<String> {
    std::fs::read_to_string(key).ok()
}

#[cfg(not(target_arch = "wasm32"))]
pub fn write(key: &str, text: &str) -> Result<(), String> {
    std::fs::write(key, text).map_err(|e| e.to_string())
}

#[cfg(target_arch = "wasm32")]
fn local() -> Option<web_sys::Storage> {
    web_sys::window()?.local_storage().ok()?
}

#[cfg(target_arch = "wasm32")]
pub fn read(key: &str) -> Option<String> {
    local()?.get_item(key).ok()?
}

#[cfg(target_arch = "wasm32")]
pub fn write(key: &str, text: &str) -> Result<(), String> {
    local()
        .ok_or("localStorage unavailable")?
        .set_item(key, text)
        .map_err(|e| format!("{e:?}"))
}
