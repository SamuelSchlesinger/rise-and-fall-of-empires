//! Colour themes. The interface is drawn once in its native palette and a
//! theme then re-maps every cell's colours, so a monochrome phosphor look
//! or a light paper look costs nothing to add.

use crate::term::Rgb;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Theme {
    Default,
    Phosphor,
    Amber,
    Paper,
    Dusk,
}

impl Theme {
    pub fn all() -> [Theme; 5] {
        [
            Theme::Default,
            Theme::Phosphor,
            Theme::Amber,
            Theme::Paper,
            Theme::Dusk,
        ]
    }
    pub fn name(self) -> &'static str {
        match self {
            Theme::Default => "default",
            Theme::Phosphor => "phosphor",
            Theme::Amber => "amber",
            Theme::Paper => "paper",
            Theme::Dusk => "dusk",
        }
    }
    pub fn from_name(s: &str) -> Option<Theme> {
        let s = s.to_lowercase();
        Theme::all().into_iter().find(|t| t.name() == s)
    }
    pub fn next(self) -> Theme {
        let all = Theme::all();
        let i = all.iter().position(|&t| t == self).unwrap_or(0);
        all[(i + 1) % all.len()]
    }
    pub fn is_identity(self) -> bool {
        self == Theme::Default
    }

    /// Re-map a colour. `bg` says whether it paints a background cell.
    pub fn map(self, c: Rgb, bg: bool) -> Rgb {
        match self {
            Theme::Default => c,
            Theme::Phosphor => mono(c, bg, Rgb(40, 255, 90)),
            Theme::Amber => mono(c, bg, Rgb(255, 190, 60)),
            Theme::Paper => {
                let cream = Rgb(246, 240, 224);
                let ink = Rgb(48, 36, 26);
                let (h, sat, v) = c.to_hsv();
                if bg {
                    if sat < 0.12 {
                        // Neutral chrome becomes shades of paper.
                        cream.scale(0.9 + v * 0.1)
                    } else {
                        // Coloured cells become pastels that keep their hue.
                        Rgb::from_hsv(h, (sat * 0.45).min(0.5), 0.9 + (1.0 - v) * 0.06)
                            .mix(cream, 0.2)
                    }
                } else if sat < 0.15 {
                    ink.mix(Rgb(120, 110, 100), (1.0 - v) * 0.8)
                } else {
                    Rgb::from_hsv(h, (sat * 1.1).min(1.0), (0.3 + v * 0.25).min(0.6))
                }
            }
            Theme::Dusk => {
                // Cooler, dimmer palette for late-night watching.
                let l = c.luma();
                let cool = c.mix(Rgb(90, 100, 150), 0.25);
                if bg {
                    cool.scale(0.8)
                } else {
                    cool.scale(0.9 + l * 0.1)
                }
            }
        }
    }
}

fn mono(c: Rgb, bg: bool, tint: Rgb) -> Rgb {
    let l = c.luma();
    let k = if bg { l * 0.45 } else { 0.25 + l * 0.9 };
    Rgb(
        (tint.0 as f32 * k).clamp(0.0, 255.0) as u8,
        (tint.1 as f32 * k).clamp(0.0, 255.0) as u8,
        (tint.2 as f32 * k).clamp(0.0, 255.0) as u8,
    )
}
