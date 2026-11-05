#[cfg(feature = "scene")]
pub mod scene;
pub mod shell;

pub use shell::{EditorModule, EditorShell};

pub fn write_atomic(path: &std::path::Path, text: &str) -> Result<(), String> {
    let mut temp = path.as_os_str().to_owned();
    temp.push(".tmp");
    let temp = std::path::PathBuf::from(temp);
    std::fs::write(&temp, text).map_err(|e| e.to_string())?;
    std::fs::rename(&temp, path).map_err(|e| e.to_string())
}

#[cfg(test)]
mod save_tests {
    use super::write_atomic;

    #[test]
    fn atomic_write_replaces_the_file_and_leaves_no_temp() {
        let dir = std::env::temp_dir().join(format!("tungsten-save-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("scene.toml");
        std::fs::write(&path, "old").unwrap();
        write_atomic(&path, "new").unwrap();
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "new");
        assert!(!dir.join("scene.toml.tmp").exists());
        std::fs::remove_dir_all(dir).unwrap();
    }
}
