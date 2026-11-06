use crate::sprites::Sprites;
use crate::texture::Texture2D;

const SCALE: u32 = 4;
const BRAZIL: [&str; 11] = [
    "gggggggggggggggg",
    "gggggggyyggggggg",
    "gggggyyyyyyggggg",
    "gggyyyybbyyyyggg",
    "gyyyybbbbbbyyyyg",
    "yyyybbwwwwbbyyyy",
    "gyyyybbbbbbyyyyg",
    "gggyyyybbyyyyggg",
    "gggggyyyyyyggggg",
    "gggggggyyggggggg",
    "gggggggggggggggg",
];
const UNITED_STATES: [&str; 11] = [
    "nwnwnwnrrrrrrrrr",
    "nnnnnnnwwwwwwwww",
    "nwnwnwnrrrrrrrrr",
    "nnnnnnnwwwwwwwww",
    "nwnwnwnrrrrrrrrr",
    "nnnnnnnwwwwwwwww",
    "rrrrrrrrrrrrrrrr",
    "wwwwwwwwwwwwwwww",
    "rrrrrrrrrrrrrrrr",
    "wwwwwwwwwwwwwwww",
    "rrrrrrrrrrrrrrrr",
];

fn color(code: u8) -> [u8; 3] {
    match code {
        b'g' => [28, 140, 72],
        b'y' => [248, 208, 48],
        b'b' => [32, 64, 148],
        b'n' => [44, 60, 128],
        b'r' => [200, 44, 56],
        _ => [244, 244, 240],
    }
}

fn flag(rows: [&str; 11]) -> Texture2D {
    let (width, height) = (16 * SCALE, 11 * SCALE);
    let mut rgba = Vec::with_capacity((width * height * 4) as usize);
    for y in 0..height {
        for x in 0..width {
            let code = rows[(y / SCALE) as usize].as_bytes()[(x / SCALE) as usize];
            let [r, g, b] = color(code);
            rgba.extend([r, g, b, 255]);
        }
    }
    Texture2D {
        width,
        height,
        rgba,
    }
}

pub fn install(sprites: &mut Sprites) {
    for (locale, rows) in [("pt-BR", BRAZIL), ("en-US", UNITED_STATES)] {
        let name = format!("flag-{locale}");
        if !sprites.has(name.as_str()) {
            sprites.insert(&name, flag(rows));
        }
    }
}
