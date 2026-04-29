#![allow(dead_code)]

use gpui::{Pixels, Rgba};

pub const S1: Pixels = gpui::px(2.0);
pub const S2: Pixels = gpui::px(4.0);
pub const S3: Pixels = gpui::px(8.0);
pub const S4: Pixels = gpui::px(16.0);
pub const S5: Pixels = gpui::px(32.0);
pub const S6: Pixels = gpui::px(260.0);

pub const GREEN1: Rgba = rgba(0x030907);
pub const GREEN2: Rgba = rgba(0x071d10);
pub const GREEN3: Rgba = rgba(0x082208);
pub const GREEN4: Rgba = rgba(0x142909);
pub const GREEN5: Rgba = rgba(0x30371a);
pub const GREEN6: Rgba = rgba(0x366317);
pub const GREEN7: Rgba = rgba(0x0aca1a);

pub const GRAY1: Rgba = rgba(0x131610);
pub const GRAY2: Rgba = rgba(0x2c2826);
pub const GRAY3: Rgba = rgba(0x57524f);
pub const GRAY4: Rgba = rgba(0x807672);
pub const GRAY5: Rgba = rgba(0xb0a69a);
pub const GRAY6: Rgba = rgba(0xe0d6ca);

pub const YELLOW1: Rgba = rgba(0x161303);
pub const YELLOW2: Rgba = rgba(0x302507);
pub const YELLOW3: Rgba = rgba(0x5a4f0e);
pub const YELLOW4: Rgba = rgba(0x837339);
pub const YELLOW5: Rgba = rgba(0xb39f4b);
pub const YELLOW6: Rgba = rgba(0xe3d34b);

pub const BLUE1: Rgba = rgba(0x175cfe);
pub const BLUE2: Rgba = rgba(0x0abab5);

pub const RED1: Rgba = rgba(0x651a20);
pub const RED2: Rgba = rgba(0xf21d23);

pub const WHITE: Rgba = rgba(0xffffff);

const fn rgba(hex: u32) -> Rgba {
    let r = ((hex >> 16) & 0xff) as f32 / 255.0;
    let g = ((hex >> 8) & 0xff) as f32 / 255.0;
    let b = (hex & 0xff) as f32 / 255.0;

    Rgba { r, g, b, a: 1.0 }
}
