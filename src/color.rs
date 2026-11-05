fn srgb_to_linear(c: f32) -> f32 {
    if c <= 0.04045 {
        c / 12.92
    } else {
        ((c + 0.055) / 1.055).powf(2.4)
    }
}

fn linear_to_srgb(c: f32) -> f32 {
    if c <= 0.0031308 {
        c * 12.92
    } else {
        1.055 * c.powf(1.0 / 2.4) - 0.055
    }
}

fn hsl_to_rgb(h: f32, s: f32, l: f32) -> (f32, f32, f32) {
    let c = (1.0 - (2.0 * l - 1.0).abs()) * s;
    let hp = (h.rem_euclid(360.0)) / 60.0;
    let x = c * (1.0 - (hp % 2.0 - 1.0).abs());
    let (r, g, b) = match hp as i32 {
        0 => (c, x, 0.0),
        1 => (x, c, 0.0),
        2 => (0.0, c, x),
        3 => (0.0, x, c),
        4 => (x, 0.0, c),
        _ => (c, 0.0, x),
    };
    let m = l - c / 2.0;
    (r + m, g + m, b + m)
}

impl serde::Serialize for Color {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let alpha = (self.a.clamp(0.0, 1.0) * 255.0).round() as u8;
        serializer.serialize_str(&format!("#{}{alpha:02x}", self.to_hex()))
    }
}

impl<'de> serde::Deserialize<'de> for Color {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let text = String::deserialize(deserializer)?;
        Ok(Color::from_name(&text).unwrap_or_else(|| Color::hex(&text)))
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Color {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
}

impl Color {
    pub const WHITE: Color = Color::rgb(255, 255, 255);
    pub const BLACK: Color = Color::rgb(0, 0, 0);
    pub const RED: Color = Color::rgb(230, 60, 60);
    pub const GREEN: Color = Color::rgb(70, 200, 90);
    pub const BLUE: Color = Color::rgb(70, 120, 230);
    pub const YELLOW: Color = Color::rgb(240, 215, 90);
    pub const ORANGE: Color = Color::rgb(240, 140, 50);
    pub const CYAN: Color = Color::rgb(70, 210, 220);
    pub const MAGENTA: Color = Color::rgb(220, 70, 200);
    pub const GRAY: Color = Color::rgb(130, 130, 140);
    pub const CLEAR: Color = Color::rgba(0, 0, 0, 0);
    pub const MISSING: Color = Color::rgb(255, 0, 255);

    // semantic text tokens
    pub const TEXT_DEFAULT: Color = Color::SLATE_200;
    pub const TEXT_MUTED: Color = Color::SLATE_400;
    pub const TEXT_TITLE: Color = Color::AMBER_200;

    // Tailwind v4 palette
    pub const SLATE_50: Color = Color::rgb(248, 250, 252);
    pub const SLATE_100: Color = Color::rgb(241, 245, 249);
    pub const SLATE_200: Color = Color::rgb(226, 232, 240);
    pub const SLATE_300: Color = Color::rgb(202, 213, 226);
    pub const SLATE_400: Color = Color::rgb(144, 161, 185);
    pub const SLATE_500: Color = Color::rgb(98, 116, 142);
    pub const SLATE_600: Color = Color::rgb(69, 85, 108);
    pub const SLATE_700: Color = Color::rgb(49, 65, 88);
    pub const SLATE_800: Color = Color::rgb(29, 41, 61);
    pub const SLATE_900: Color = Color::rgb(15, 23, 43);
    pub const SLATE_950: Color = Color::rgb(2, 6, 24);
    pub const GRAY_50: Color = Color::rgb(249, 250, 251);
    pub const GRAY_100: Color = Color::rgb(243, 244, 246);
    pub const GRAY_200: Color = Color::rgb(229, 231, 235);
    pub const GRAY_300: Color = Color::rgb(209, 213, 220);
    pub const GRAY_400: Color = Color::rgb(153, 161, 175);
    pub const GRAY_500: Color = Color::rgb(106, 114, 130);
    pub const GRAY_600: Color = Color::rgb(74, 85, 101);
    pub const GRAY_700: Color = Color::rgb(54, 65, 83);
    pub const GRAY_800: Color = Color::rgb(30, 41, 57);
    pub const GRAY_900: Color = Color::rgb(16, 24, 40);
    pub const GRAY_950: Color = Color::rgb(3, 7, 18);
    pub const RED_50: Color = Color::rgb(254, 242, 242);
    pub const RED_100: Color = Color::rgb(255, 226, 226);
    pub const RED_200: Color = Color::rgb(255, 201, 201);
    pub const RED_300: Color = Color::rgb(255, 162, 162);
    pub const RED_400: Color = Color::rgb(255, 100, 103);
    pub const RED_500: Color = Color::rgb(251, 44, 54);
    pub const RED_600: Color = Color::rgb(231, 0, 11);
    pub const RED_700: Color = Color::rgb(193, 0, 7);
    pub const RED_800: Color = Color::rgb(159, 7, 18);
    pub const RED_900: Color = Color::rgb(130, 24, 26);
    pub const RED_950: Color = Color::rgb(70, 8, 9);
    pub const ORANGE_50: Color = Color::rgb(255, 247, 237);
    pub const ORANGE_100: Color = Color::rgb(255, 237, 212);
    pub const ORANGE_200: Color = Color::rgb(255, 214, 167);
    pub const ORANGE_300: Color = Color::rgb(255, 184, 106);
    pub const ORANGE_400: Color = Color::rgb(255, 137, 4);
    pub const ORANGE_500: Color = Color::rgb(255, 105, 0);
    pub const ORANGE_600: Color = Color::rgb(245, 73, 0);
    pub const ORANGE_700: Color = Color::rgb(202, 53, 0);
    pub const ORANGE_800: Color = Color::rgb(159, 45, 0);
    pub const ORANGE_900: Color = Color::rgb(126, 42, 12);
    pub const ORANGE_950: Color = Color::rgb(68, 19, 6);
    pub const AMBER_50: Color = Color::rgb(255, 251, 235);
    pub const AMBER_100: Color = Color::rgb(254, 243, 198);
    pub const AMBER_200: Color = Color::rgb(254, 230, 133);
    pub const AMBER_300: Color = Color::rgb(255, 210, 48);
    pub const AMBER_400: Color = Color::rgb(255, 185, 0);
    pub const AMBER_500: Color = Color::rgb(254, 154, 0);
    pub const AMBER_600: Color = Color::rgb(225, 113, 0);
    pub const AMBER_700: Color = Color::rgb(187, 77, 0);
    pub const AMBER_800: Color = Color::rgb(151, 60, 0);
    pub const AMBER_900: Color = Color::rgb(123, 51, 6);
    pub const AMBER_950: Color = Color::rgb(70, 25, 1);
    pub const YELLOW_50: Color = Color::rgb(254, 252, 232);
    pub const YELLOW_100: Color = Color::rgb(254, 249, 194);
    pub const YELLOW_200: Color = Color::rgb(255, 240, 133);
    pub const YELLOW_300: Color = Color::rgb(255, 223, 32);
    pub const YELLOW_400: Color = Color::rgb(253, 199, 0);
    pub const YELLOW_500: Color = Color::rgb(240, 177, 0);
    pub const YELLOW_600: Color = Color::rgb(208, 135, 0);
    pub const YELLOW_700: Color = Color::rgb(166, 95, 0);
    pub const YELLOW_800: Color = Color::rgb(137, 75, 0);
    pub const YELLOW_900: Color = Color::rgb(115, 62, 10);
    pub const YELLOW_950: Color = Color::rgb(67, 32, 4);
    pub const LIME_50: Color = Color::rgb(247, 254, 231);
    pub const LIME_100: Color = Color::rgb(236, 252, 202);
    pub const LIME_200: Color = Color::rgb(216, 249, 153);
    pub const LIME_300: Color = Color::rgb(187, 244, 81);
    pub const LIME_400: Color = Color::rgb(154, 230, 0);
    pub const LIME_500: Color = Color::rgb(124, 207, 0);
    pub const LIME_600: Color = Color::rgb(94, 165, 0);
    pub const LIME_700: Color = Color::rgb(73, 125, 0);
    pub const LIME_800: Color = Color::rgb(60, 99, 0);
    pub const LIME_900: Color = Color::rgb(53, 83, 14);
    pub const LIME_950: Color = Color::rgb(25, 46, 3);
    pub const GREEN_50: Color = Color::rgb(240, 253, 244);
    pub const GREEN_100: Color = Color::rgb(220, 252, 231);
    pub const GREEN_200: Color = Color::rgb(185, 248, 207);
    pub const GREEN_300: Color = Color::rgb(123, 241, 168);
    pub const GREEN_400: Color = Color::rgb(5, 223, 114);
    pub const GREEN_500: Color = Color::rgb(0, 201, 80);
    pub const GREEN_600: Color = Color::rgb(0, 166, 62);
    pub const GREEN_700: Color = Color::rgb(0, 130, 54);
    pub const GREEN_800: Color = Color::rgb(1, 102, 48);
    pub const GREEN_900: Color = Color::rgb(13, 84, 43);
    pub const GREEN_950: Color = Color::rgb(3, 46, 21);
    pub const EMERALD_50: Color = Color::rgb(236, 253, 245);
    pub const EMERALD_100: Color = Color::rgb(208, 250, 229);
    pub const EMERALD_200: Color = Color::rgb(164, 244, 207);
    pub const EMERALD_300: Color = Color::rgb(94, 233, 181);
    pub const EMERALD_400: Color = Color::rgb(0, 212, 146);
    pub const EMERALD_500: Color = Color::rgb(0, 188, 125);
    pub const EMERALD_600: Color = Color::rgb(0, 153, 102);
    pub const EMERALD_700: Color = Color::rgb(0, 122, 85);
    pub const EMERALD_800: Color = Color::rgb(0, 96, 69);
    pub const EMERALD_900: Color = Color::rgb(0, 79, 59);
    pub const EMERALD_950: Color = Color::rgb(0, 44, 34);
    pub const TEAL_50: Color = Color::rgb(240, 253, 250);
    pub const TEAL_100: Color = Color::rgb(203, 251, 241);
    pub const TEAL_200: Color = Color::rgb(150, 247, 228);
    pub const TEAL_300: Color = Color::rgb(70, 236, 213);
    pub const TEAL_400: Color = Color::rgb(0, 213, 190);
    pub const TEAL_500: Color = Color::rgb(0, 187, 167);
    pub const TEAL_600: Color = Color::rgb(0, 150, 137);
    pub const TEAL_700: Color = Color::rgb(0, 120, 111);
    pub const TEAL_800: Color = Color::rgb(0, 95, 90);
    pub const TEAL_900: Color = Color::rgb(11, 79, 74);
    pub const TEAL_950: Color = Color::rgb(2, 47, 46);
    pub const CYAN_50: Color = Color::rgb(236, 254, 255);
    pub const CYAN_100: Color = Color::rgb(206, 250, 254);
    pub const CYAN_200: Color = Color::rgb(162, 244, 253);
    pub const CYAN_300: Color = Color::rgb(83, 234, 253);
    pub const CYAN_400: Color = Color::rgb(0, 211, 242);
    pub const CYAN_500: Color = Color::rgb(0, 184, 219);
    pub const CYAN_600: Color = Color::rgb(0, 146, 184);
    pub const CYAN_700: Color = Color::rgb(0, 117, 149);
    pub const CYAN_800: Color = Color::rgb(0, 95, 120);
    pub const CYAN_900: Color = Color::rgb(16, 78, 100);
    pub const CYAN_950: Color = Color::rgb(5, 51, 69);
    pub const SKY_50: Color = Color::rgb(240, 249, 255);
    pub const SKY_100: Color = Color::rgb(223, 242, 254);
    pub const SKY_200: Color = Color::rgb(184, 230, 254);
    pub const SKY_300: Color = Color::rgb(116, 212, 255);
    pub const SKY_400: Color = Color::rgb(0, 188, 255);
    pub const SKY_500: Color = Color::rgb(0, 166, 244);
    pub const SKY_600: Color = Color::rgb(0, 132, 209);
    pub const SKY_700: Color = Color::rgb(0, 105, 168);
    pub const SKY_800: Color = Color::rgb(0, 89, 138);
    pub const SKY_900: Color = Color::rgb(2, 74, 112);
    pub const SKY_950: Color = Color::rgb(5, 47, 74);
    pub const BLUE_50: Color = Color::rgb(239, 246, 255);
    pub const BLUE_100: Color = Color::rgb(219, 234, 254);
    pub const BLUE_200: Color = Color::rgb(190, 219, 255);
    pub const BLUE_300: Color = Color::rgb(142, 197, 255);
    pub const BLUE_400: Color = Color::rgb(81, 162, 255);
    pub const BLUE_500: Color = Color::rgb(43, 127, 255);
    pub const BLUE_600: Color = Color::rgb(21, 93, 252);
    pub const BLUE_700: Color = Color::rgb(20, 71, 230);
    pub const BLUE_800: Color = Color::rgb(25, 60, 184);
    pub const BLUE_900: Color = Color::rgb(28, 57, 142);
    pub const BLUE_950: Color = Color::rgb(22, 36, 86);
    pub const INDIGO_50: Color = Color::rgb(238, 242, 255);
    pub const INDIGO_100: Color = Color::rgb(224, 231, 255);
    pub const INDIGO_200: Color = Color::rgb(198, 210, 255);
    pub const INDIGO_300: Color = Color::rgb(163, 179, 255);
    pub const INDIGO_400: Color = Color::rgb(124, 134, 255);
    pub const INDIGO_500: Color = Color::rgb(97, 95, 255);
    pub const INDIGO_600: Color = Color::rgb(79, 57, 246);
    pub const INDIGO_700: Color = Color::rgb(67, 45, 215);
    pub const INDIGO_800: Color = Color::rgb(55, 42, 172);
    pub const INDIGO_900: Color = Color::rgb(49, 44, 133);
    pub const INDIGO_950: Color = Color::rgb(30, 26, 77);
    pub const VIOLET_50: Color = Color::rgb(245, 243, 255);
    pub const VIOLET_100: Color = Color::rgb(237, 233, 254);
    pub const VIOLET_200: Color = Color::rgb(221, 214, 255);
    pub const VIOLET_300: Color = Color::rgb(196, 180, 255);
    pub const VIOLET_400: Color = Color::rgb(166, 132, 255);
    pub const VIOLET_500: Color = Color::rgb(142, 81, 255);
    pub const VIOLET_600: Color = Color::rgb(127, 34, 254);
    pub const VIOLET_700: Color = Color::rgb(112, 8, 231);
    pub const VIOLET_800: Color = Color::rgb(93, 14, 192);
    pub const VIOLET_900: Color = Color::rgb(77, 23, 154);
    pub const VIOLET_950: Color = Color::rgb(47, 13, 104);
    pub const PURPLE_50: Color = Color::rgb(250, 245, 255);
    pub const PURPLE_100: Color = Color::rgb(243, 232, 255);
    pub const PURPLE_200: Color = Color::rgb(233, 212, 255);
    pub const PURPLE_300: Color = Color::rgb(218, 178, 255);
    pub const PURPLE_400: Color = Color::rgb(194, 122, 255);
    pub const PURPLE_500: Color = Color::rgb(173, 70, 255);
    pub const PURPLE_600: Color = Color::rgb(152, 16, 250);
    pub const PURPLE_700: Color = Color::rgb(130, 0, 219);
    pub const PURPLE_800: Color = Color::rgb(110, 17, 176);
    pub const PURPLE_900: Color = Color::rgb(89, 22, 139);
    pub const PURPLE_950: Color = Color::rgb(60, 3, 102);
    pub const FUCHSIA_50: Color = Color::rgb(253, 244, 255);
    pub const FUCHSIA_100: Color = Color::rgb(250, 232, 255);
    pub const FUCHSIA_200: Color = Color::rgb(246, 207, 255);
    pub const FUCHSIA_300: Color = Color::rgb(244, 168, 255);
    pub const FUCHSIA_400: Color = Color::rgb(237, 106, 255);
    pub const FUCHSIA_500: Color = Color::rgb(225, 42, 251);
    pub const FUCHSIA_600: Color = Color::rgb(200, 0, 222);
    pub const FUCHSIA_700: Color = Color::rgb(168, 0, 183);
    pub const FUCHSIA_800: Color = Color::rgb(138, 1, 148);
    pub const FUCHSIA_900: Color = Color::rgb(114, 19, 120);
    pub const FUCHSIA_950: Color = Color::rgb(75, 0, 79);
    pub const PINK_50: Color = Color::rgb(253, 242, 248);
    pub const PINK_100: Color = Color::rgb(252, 231, 243);
    pub const PINK_200: Color = Color::rgb(252, 206, 232);
    pub const PINK_300: Color = Color::rgb(253, 165, 213);
    pub const PINK_400: Color = Color::rgb(251, 100, 182);
    pub const PINK_500: Color = Color::rgb(246, 51, 154);
    pub const PINK_600: Color = Color::rgb(230, 0, 118);
    pub const PINK_700: Color = Color::rgb(198, 0, 92);
    pub const PINK_800: Color = Color::rgb(163, 0, 76);
    pub const PINK_900: Color = Color::rgb(134, 16, 67);
    pub const PINK_950: Color = Color::rgb(81, 4, 36);
    pub const ROSE_50: Color = Color::rgb(255, 241, 242);
    pub const ROSE_100: Color = Color::rgb(255, 228, 230);
    pub const ROSE_200: Color = Color::rgb(255, 204, 211);
    pub const ROSE_300: Color = Color::rgb(255, 161, 173);
    pub const ROSE_400: Color = Color::rgb(255, 99, 126);
    pub const ROSE_500: Color = Color::rgb(255, 32, 86);
    pub const ROSE_600: Color = Color::rgb(236, 0, 63);
    pub const ROSE_700: Color = Color::rgb(199, 0, 54);
    pub const ROSE_800: Color = Color::rgb(165, 0, 54);
    pub const ROSE_900: Color = Color::rgb(139, 8, 54);
    pub const ROSE_950: Color = Color::rgb(77, 2, 24);

    pub const fn new(r: f32, g: f32, b: f32, a: f32) -> Self {
        Color { r, g, b, a }
    }

    pub const fn rgba(r: u8, g: u8, b: u8, a: u8) -> Self {
        Color::new(
            r as f32 / 255.0,
            g as f32 / 255.0,
            b as f32 / 255.0,
            a as f32 / 255.0,
        )
    }

    pub const fn rgb(r: u8, g: u8, b: u8) -> Self {
        Self::rgba(r, g, b, 255)
    }

    pub fn from_linear(rgba: [f32; 4]) -> Self {
        Color::new(
            linear_to_srgb(rgba[0]),
            linear_to_srgb(rgba[1]),
            linear_to_srgb(rgba[2]),
            rgba[3],
        )
    }

    pub fn hsla(h: f32, s: f32, l: f32, a: f32) -> Self {
        let (r, g, b) = hsl_to_rgb(h, s, l);
        Color::new(r, g, b, a)
    }

    pub fn hsl(h: f32, s: f32, l: f32) -> Self {
        Self::hsla(h, s, l, 1.0)
    }

    pub fn hsv(h: f32, s: f32, v: f32) -> Self {
        let c = v * s;
        let hp = h.rem_euclid(360.0) / 60.0;
        let x = c * (1.0 - (hp % 2.0 - 1.0).abs());
        let (r, g, b) = match hp as i32 {
            0 => (c, x, 0.0),
            1 => (x, c, 0.0),
            2 => (0.0, c, x),
            3 => (0.0, x, c),
            4 => (x, 0.0, c),
            _ => (c, 0.0, x),
        };
        let m = v - c;
        Color::new(r + m, g + m, b + m, 1.0)
    }

    pub fn to_hsv(self) -> (f32, f32, f32) {
        let max = self.r.max(self.g).max(self.b);
        let min = self.r.min(self.g).min(self.b);
        let d = max - min;
        let h = if d <= f32::EPSILON {
            0.0
        } else if max == self.r {
            60.0 * ((self.g - self.b) / d).rem_euclid(6.0)
        } else if max == self.g {
            60.0 * ((self.b - self.r) / d + 2.0)
        } else {
            60.0 * ((self.r - self.g) / d + 4.0)
        };
        let s = if max <= f32::EPSILON { 0.0 } else { d / max };
        (h, s, max)
    }

    pub fn to_hex(self) -> String {
        let b = |v: f32| (v.clamp(0.0, 1.0) * 255.0).round() as u8;
        format!("{:02x}{:02x}{:02x}", b(self.r), b(self.g), b(self.b))
    }

    pub fn hex(s: &str) -> Self {
        let s = s.strip_prefix('#').unwrap_or(s);
        let byte = |i: usize| u8::from_str_radix(&s[i..i + 2], 16).ok();
        match s.len() {
            6 => match (byte(0), byte(2), byte(4)) {
                (Some(r), Some(g), Some(b)) => Self::rgb(r, g, b),
                _ => Self::MISSING,
            },
            8 => match (byte(0), byte(2), byte(4), byte(6)) {
                (Some(r), Some(g), Some(b), Some(a)) => Self::rgba(r, g, b, a),
                _ => Self::MISSING,
            },
            _ => Self::MISSING,
        }
    }

    pub fn from_name(name: &str) -> Option<Color> {
        match name {
            "white" => Some(Color::WHITE),
            "black" => Some(Color::BLACK),
            "red" => Some(Color::RED),
            "green" => Some(Color::GREEN),
            "blue" => Some(Color::BLUE),
            "yellow" => Some(Color::YELLOW),
            "orange" => Some(Color::ORANGE),
            "cyan" => Some(Color::CYAN),
            "gray" => Some(Color::GRAY),
            "slate-50" => Some(Color::SLATE_50),
            "slate-100" => Some(Color::SLATE_100),
            "slate-200" => Some(Color::SLATE_200),
            "slate-300" => Some(Color::SLATE_300),
            "slate-400" => Some(Color::SLATE_400),
            "slate-500" => Some(Color::SLATE_500),
            "slate-600" => Some(Color::SLATE_600),
            "slate-700" => Some(Color::SLATE_700),
            "slate-800" => Some(Color::SLATE_800),
            "slate-900" => Some(Color::SLATE_900),
            "slate-950" => Some(Color::SLATE_950),
            "gray-50" => Some(Color::GRAY_50),
            "gray-100" => Some(Color::GRAY_100),
            "gray-200" => Some(Color::GRAY_200),
            "gray-300" => Some(Color::GRAY_300),
            "gray-400" => Some(Color::GRAY_400),
            "gray-500" => Some(Color::GRAY_500),
            "gray-600" => Some(Color::GRAY_600),
            "gray-700" => Some(Color::GRAY_700),
            "gray-800" => Some(Color::GRAY_800),
            "gray-900" => Some(Color::GRAY_900),
            "gray-950" => Some(Color::GRAY_950),
            "red-50" => Some(Color::RED_50),
            "red-100" => Some(Color::RED_100),
            "red-200" => Some(Color::RED_200),
            "red-300" => Some(Color::RED_300),
            "red-400" => Some(Color::RED_400),
            "red-500" => Some(Color::RED_500),
            "red-600" => Some(Color::RED_600),
            "red-700" => Some(Color::RED_700),
            "red-800" => Some(Color::RED_800),
            "red-900" => Some(Color::RED_900),
            "red-950" => Some(Color::RED_950),
            "orange-50" => Some(Color::ORANGE_50),
            "orange-100" => Some(Color::ORANGE_100),
            "orange-200" => Some(Color::ORANGE_200),
            "orange-300" => Some(Color::ORANGE_300),
            "orange-400" => Some(Color::ORANGE_400),
            "orange-500" => Some(Color::ORANGE_500),
            "orange-600" => Some(Color::ORANGE_600),
            "orange-700" => Some(Color::ORANGE_700),
            "orange-800" => Some(Color::ORANGE_800),
            "orange-900" => Some(Color::ORANGE_900),
            "orange-950" => Some(Color::ORANGE_950),
            "amber-50" => Some(Color::AMBER_50),
            "amber-100" => Some(Color::AMBER_100),
            "amber-200" => Some(Color::AMBER_200),
            "amber-300" => Some(Color::AMBER_300),
            "amber-400" => Some(Color::AMBER_400),
            "amber-500" => Some(Color::AMBER_500),
            "amber-600" => Some(Color::AMBER_600),
            "amber-700" => Some(Color::AMBER_700),
            "amber-800" => Some(Color::AMBER_800),
            "amber-900" => Some(Color::AMBER_900),
            "amber-950" => Some(Color::AMBER_950),
            "yellow-50" => Some(Color::YELLOW_50),
            "yellow-100" => Some(Color::YELLOW_100),
            "yellow-200" => Some(Color::YELLOW_200),
            "yellow-300" => Some(Color::YELLOW_300),
            "yellow-400" => Some(Color::YELLOW_400),
            "yellow-500" => Some(Color::YELLOW_500),
            "yellow-600" => Some(Color::YELLOW_600),
            "yellow-700" => Some(Color::YELLOW_700),
            "yellow-800" => Some(Color::YELLOW_800),
            "yellow-900" => Some(Color::YELLOW_900),
            "yellow-950" => Some(Color::YELLOW_950),
            "lime-50" => Some(Color::LIME_50),
            "lime-100" => Some(Color::LIME_100),
            "lime-200" => Some(Color::LIME_200),
            "lime-300" => Some(Color::LIME_300),
            "lime-400" => Some(Color::LIME_400),
            "lime-500" => Some(Color::LIME_500),
            "lime-600" => Some(Color::LIME_600),
            "lime-700" => Some(Color::LIME_700),
            "lime-800" => Some(Color::LIME_800),
            "lime-900" => Some(Color::LIME_900),
            "lime-950" => Some(Color::LIME_950),
            "green-50" => Some(Color::GREEN_50),
            "green-100" => Some(Color::GREEN_100),
            "green-200" => Some(Color::GREEN_200),
            "green-300" => Some(Color::GREEN_300),
            "green-400" => Some(Color::GREEN_400),
            "green-500" => Some(Color::GREEN_500),
            "green-600" => Some(Color::GREEN_600),
            "green-700" => Some(Color::GREEN_700),
            "green-800" => Some(Color::GREEN_800),
            "green-900" => Some(Color::GREEN_900),
            "green-950" => Some(Color::GREEN_950),
            "emerald-50" => Some(Color::EMERALD_50),
            "emerald-100" => Some(Color::EMERALD_100),
            "emerald-200" => Some(Color::EMERALD_200),
            "emerald-300" => Some(Color::EMERALD_300),
            "emerald-400" => Some(Color::EMERALD_400),
            "emerald-500" => Some(Color::EMERALD_500),
            "emerald-600" => Some(Color::EMERALD_600),
            "emerald-700" => Some(Color::EMERALD_700),
            "emerald-800" => Some(Color::EMERALD_800),
            "emerald-900" => Some(Color::EMERALD_900),
            "emerald-950" => Some(Color::EMERALD_950),
            "teal-50" => Some(Color::TEAL_50),
            "teal-100" => Some(Color::TEAL_100),
            "teal-200" => Some(Color::TEAL_200),
            "teal-300" => Some(Color::TEAL_300),
            "teal-400" => Some(Color::TEAL_400),
            "teal-500" => Some(Color::TEAL_500),
            "teal-600" => Some(Color::TEAL_600),
            "teal-700" => Some(Color::TEAL_700),
            "teal-800" => Some(Color::TEAL_800),
            "teal-900" => Some(Color::TEAL_900),
            "teal-950" => Some(Color::TEAL_950),
            "cyan-50" => Some(Color::CYAN_50),
            "cyan-100" => Some(Color::CYAN_100),
            "cyan-200" => Some(Color::CYAN_200),
            "cyan-300" => Some(Color::CYAN_300),
            "cyan-400" => Some(Color::CYAN_400),
            "cyan-500" => Some(Color::CYAN_500),
            "cyan-600" => Some(Color::CYAN_600),
            "cyan-700" => Some(Color::CYAN_700),
            "cyan-800" => Some(Color::CYAN_800),
            "cyan-900" => Some(Color::CYAN_900),
            "cyan-950" => Some(Color::CYAN_950),
            "sky-50" => Some(Color::SKY_50),
            "sky-100" => Some(Color::SKY_100),
            "sky-200" => Some(Color::SKY_200),
            "sky-300" => Some(Color::SKY_300),
            "sky-400" => Some(Color::SKY_400),
            "sky-500" => Some(Color::SKY_500),
            "sky-600" => Some(Color::SKY_600),
            "sky-700" => Some(Color::SKY_700),
            "sky-800" => Some(Color::SKY_800),
            "sky-900" => Some(Color::SKY_900),
            "sky-950" => Some(Color::SKY_950),
            "blue-50" => Some(Color::BLUE_50),
            "blue-100" => Some(Color::BLUE_100),
            "blue-200" => Some(Color::BLUE_200),
            "blue-300" => Some(Color::BLUE_300),
            "blue-400" => Some(Color::BLUE_400),
            "blue-500" => Some(Color::BLUE_500),
            "blue-600" => Some(Color::BLUE_600),
            "blue-700" => Some(Color::BLUE_700),
            "blue-800" => Some(Color::BLUE_800),
            "blue-900" => Some(Color::BLUE_900),
            "blue-950" => Some(Color::BLUE_950),
            "indigo-50" => Some(Color::INDIGO_50),
            "indigo-100" => Some(Color::INDIGO_100),
            "indigo-200" => Some(Color::INDIGO_200),
            "indigo-300" => Some(Color::INDIGO_300),
            "indigo-400" => Some(Color::INDIGO_400),
            "indigo-500" => Some(Color::INDIGO_500),
            "indigo-600" => Some(Color::INDIGO_600),
            "indigo-700" => Some(Color::INDIGO_700),
            "indigo-800" => Some(Color::INDIGO_800),
            "indigo-900" => Some(Color::INDIGO_900),
            "indigo-950" => Some(Color::INDIGO_950),
            "violet-50" => Some(Color::VIOLET_50),
            "violet-100" => Some(Color::VIOLET_100),
            "violet-200" => Some(Color::VIOLET_200),
            "violet-300" => Some(Color::VIOLET_300),
            "violet-400" => Some(Color::VIOLET_400),
            "violet-500" => Some(Color::VIOLET_500),
            "violet-600" => Some(Color::VIOLET_600),
            "violet-700" => Some(Color::VIOLET_700),
            "violet-800" => Some(Color::VIOLET_800),
            "violet-900" => Some(Color::VIOLET_900),
            "violet-950" => Some(Color::VIOLET_950),
            "purple-50" => Some(Color::PURPLE_50),
            "purple-100" => Some(Color::PURPLE_100),
            "purple-200" => Some(Color::PURPLE_200),
            "purple-300" => Some(Color::PURPLE_300),
            "purple-400" => Some(Color::PURPLE_400),
            "purple-500" => Some(Color::PURPLE_500),
            "purple-600" => Some(Color::PURPLE_600),
            "purple-700" => Some(Color::PURPLE_700),
            "purple-800" => Some(Color::PURPLE_800),
            "purple-900" => Some(Color::PURPLE_900),
            "purple-950" => Some(Color::PURPLE_950),
            "fuchsia-50" => Some(Color::FUCHSIA_50),
            "fuchsia-100" => Some(Color::FUCHSIA_100),
            "fuchsia-200" => Some(Color::FUCHSIA_200),
            "fuchsia-300" => Some(Color::FUCHSIA_300),
            "fuchsia-400" => Some(Color::FUCHSIA_400),
            "fuchsia-500" => Some(Color::FUCHSIA_500),
            "fuchsia-600" => Some(Color::FUCHSIA_600),
            "fuchsia-700" => Some(Color::FUCHSIA_700),
            "fuchsia-800" => Some(Color::FUCHSIA_800),
            "fuchsia-900" => Some(Color::FUCHSIA_900),
            "fuchsia-950" => Some(Color::FUCHSIA_950),
            "pink-50" => Some(Color::PINK_50),
            "pink-100" => Some(Color::PINK_100),
            "pink-200" => Some(Color::PINK_200),
            "pink-300" => Some(Color::PINK_300),
            "pink-400" => Some(Color::PINK_400),
            "pink-500" => Some(Color::PINK_500),
            "pink-600" => Some(Color::PINK_600),
            "pink-700" => Some(Color::PINK_700),
            "pink-800" => Some(Color::PINK_800),
            "pink-900" => Some(Color::PINK_900),
            "pink-950" => Some(Color::PINK_950),
            "rose-50" => Some(Color::ROSE_50),
            "rose-100" => Some(Color::ROSE_100),
            "rose-200" => Some(Color::ROSE_200),
            "rose-300" => Some(Color::ROSE_300),
            "rose-400" => Some(Color::ROSE_400),
            "rose-500" => Some(Color::ROSE_500),
            "rose-600" => Some(Color::ROSE_600),
            "rose-700" => Some(Color::ROSE_700),
            "rose-800" => Some(Color::ROSE_800),
            "rose-900" => Some(Color::ROSE_900),
            "rose-950" => Some(Color::ROSE_950),
            _ => None,
        }
    }

    pub fn parse(s: &str) -> Option<Color> {
        if s.starts_with('#') {
            Some(Color::hex(s))
        } else {
            Color::from_name(s)
        }
    }

    pub fn with_alpha(self, a: f32) -> Self {
        Color { a, ..self }
    }

    pub fn lerp(self, other: Color, t: f32) -> Self {
        Color::new(
            self.r + (other.r - self.r) * t,
            self.g + (other.g - self.g) * t,
            self.b + (other.b - self.b) * t,
            self.a + (other.a - self.a) * t,
        )
    }

    pub fn to_array(self) -> [f32; 4] {
        [
            srgb_to_linear(self.r),
            srgb_to_linear(self.g),
            srgb_to_linear(self.b),
            self.a,
        ]
    }
}

impl From<Color> for [f32; 4] {
    fn from(c: Color) -> [f32; 4] {
        c.to_array()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn approx(a: f32, b: f32) -> bool {
        (a - b).abs() < 1e-3
    }

    #[test]
    fn rgb_and_rgba() {
        let c = Color::rgb(255, 0, 0);
        assert!(approx(c.r, 1.0) && approx(c.g, 0.0) && approx(c.b, 0.0) && approx(c.a, 1.0));
        let c = Color::rgba(0, 0, 0, 128);
        assert!(approx(c.a, 128.0 / 255.0));
    }

    #[test]
    fn hsv_round_trip() {
        for hex in ["5fd36a", "ff9a30", "8040c0", "000000", "ffffff"] {
            let (h, s, v) = Color::hex(hex).to_hsv();
            assert_eq!(Color::hsv(h, s, v).to_hex(), hex);
        }
    }

    #[test]
    fn hsl_and_hsla() {
        let red = Color::hsl(0.0, 1.0, 0.5);
        assert!(approx(red.r, 1.0) && approx(red.g, 0.0) && approx(red.b, 0.0));
        let green = Color::hsl(120.0, 1.0, 0.5);
        assert!(approx(green.r, 0.0) && approx(green.g, 1.0) && approx(green.b, 0.0));
        assert!(approx(Color::hsla(0.0, 1.0, 0.5, 0.5).a, 0.5));
    }

    #[test]
    fn hex_parses_and_falls_back() {
        assert_eq!(Color::hex("#ff0000"), Color::rgb(255, 0, 0));
        assert_eq!(Color::hex("00ff00"), Color::rgb(0, 255, 0));
        assert!(approx(Color::hex("#000000ff").a, 1.0));
        assert!(approx(Color::hex("#00000080").a, 128.0 / 255.0));
        assert_eq!(Color::hex("nope"), Color::MISSING);
    }
}

pub mod prelude {
    pub use super::Color;
}
