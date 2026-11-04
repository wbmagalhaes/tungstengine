use std::collections::HashMap;

use serde::Deserialize;

use crate::texture::Texture2D;

pub type PaletteEntry = (char, [u8; 4]);

#[derive(Deserialize, Clone, Debug, PartialEq)]
pub struct TextSprite {
    pub rows: Vec<String>,
    pub tint: Option<String>,
}

#[derive(Deserialize, Clone, Debug, Default)]
pub struct TextSpriteSheet {
    pub sprite: HashMap<String, TextSprite>,
}

impl TextSpriteSheet {
    pub fn parse(text: &str) -> Result<TextSpriteSheet, String> {
        toml::from_str(text).map_err(|e| e.to_string())
    }
}

pub fn paint(rows: &[&str], palette: &[PaletteEntry]) -> Texture2D {
    let h = rows.len();
    let w = rows.iter().map(|r| r.chars().count()).max().unwrap_or(0);
    let mut rgba = vec![0u8; w * h * 4];
    for (y, row) in rows.iter().enumerate() {
        for (x, ch) in row.chars().enumerate() {
            if let Some((_, c)) = palette.iter().find(|(k, _)| *k == ch) {
                let i = (y * w + x) * 4;
                rgba[i..i + 4].copy_from_slice(c);
            }
        }
    }
    Texture2D {
        width: w as u32,
        height: h as u32,
        rgba,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_sheet_and_paints_palette_chars() {
        let sheet =
            TextSpriteSheet::parse("[sprite.\"a\"]\ntint = \"ff0000\"\nrows = [\"k.\", \".k\"]\n")
                .unwrap();
        let a = &sheet.sprite["a"];
        assert_eq!(a.tint.as_deref(), Some("ff0000"));
        let rows: Vec<&str> = a.rows.iter().map(String::as_str).collect();
        let t = paint(&rows, &[('k', [1, 2, 3, 255])]);
        assert_eq!((t.width, t.height), (2, 2));
        assert_eq!(&t.rgba[0..4], &[1, 2, 3, 255]);
        assert_eq!(&t.rgba[4..8], &[0, 0, 0, 0]);
    }
}
