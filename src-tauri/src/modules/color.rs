//! Color parsing, conversions (HEX, RGB, HSL, OKLCH) and WCAG contrast.

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Rgb {
    pub r: f64,
    pub g: f64,
    pub b: f64,
}

fn clamp01(v: f64) -> f64 {
    v.clamp(0.0, 1.0)
}

fn num(s: &str) -> Option<f64> {
    s.trim().trim_end_matches("deg").parse().ok()
}

/// Number or percentage, scaled so that 100% == `full`.
fn num_pct(s: &str, full: f64) -> Option<f64> {
    let s = s.trim();
    match s.strip_suffix('%') {
        Some(p) => Some(num(p)? / 100.0 * full),
        None => num(s),
    }
}

fn args<'a>(s: &'a str, name: &str) -> Option<Vec<&'a str>> {
    let inner = s.strip_prefix(name)?.trim_start().strip_prefix('(')?.strip_suffix(')')?;
    // Accept both "1, 2, 3" and "1 2 3 / 0.5"; alpha is ignored.
    let inner = inner.split('/').next()?;
    let parts: Vec<&str> = inner.split([',', ' ']).filter(|p| !p.trim().is_empty()).collect();
    (parts.len() == 3).then_some(parts)
}

pub fn parse(input: &str) -> Option<Rgb> {
    let s = input.trim().to_lowercase();
    if let Some(h) = s.strip_prefix('#') {
        let h: String = match h.len() {
            3 | 4 => h.chars().take(3).flat_map(|c| [c, c]).collect(),
            6 | 8 => h[..6].to_string(),
            _ => return None,
        };
        let v = u32::from_str_radix(&h, 16).ok()?;
        return Some(Rgb {
            r: f64::from((v >> 16) & 255) / 255.0,
            g: f64::from((v >> 8) & 255) / 255.0,
            b: f64::from(v & 255) / 255.0,
        });
    }
    if let Some(p) = args(&s, "rgba").or_else(|| args(&s, "rgb")) {
        return Some(Rgb {
            r: num_pct(p[0], 255.0)? / 255.0,
            g: num_pct(p[1], 255.0)? / 255.0,
            b: num_pct(p[2], 255.0)? / 255.0,
        });
    }
    if let Some(p) = args(&s, "hsla").or_else(|| args(&s, "hsl")) {
        return Some(hsl_to_rgb(num(p[0])?, num_pct(p[1], 100.0)? / 100.0, num_pct(p[2], 100.0)? / 100.0));
    }
    if let Some(p) = args(&s, "oklch") {
        let l = num_pct(p[0], 1.0)?;
        return Some(oklch_to_rgb(l, num_pct(p[1], 0.4)?, num(p[2])?));
    }
    None
}

fn hsl_to_rgb(h: f64, s: f64, l: f64) -> Rgb {
    let h = h.rem_euclid(360.0);
    let c = (1.0 - (2.0 * l - 1.0).abs()) * s;
    let x = c * (1.0 - ((h / 60.0) % 2.0 - 1.0).abs());
    let m = l - c / 2.0;
    let (r, g, b) = match (h / 60.0) as u32 {
        0 => (c, x, 0.0),
        1 => (x, c, 0.0),
        2 => (0.0, c, x),
        3 => (0.0, x, c),
        4 => (x, 0.0, c),
        _ => (c, 0.0, x),
    };
    Rgb { r: clamp01(r + m), g: clamp01(g + m), b: clamp01(b + m) }
}

fn to_linear(c: f64) -> f64 {
    if c <= 0.04045 { c / 12.92 } else { ((c + 0.055) / 1.055).powf(2.4) }
}

fn from_linear(c: f64) -> f64 {
    clamp01(if c <= 0.0031308 { c * 12.92 } else { 1.055 * c.powf(1.0 / 2.4) - 0.055 })
}

fn oklch_to_rgb(l: f64, c: f64, h: f64) -> Rgb {
    let (a, b) = (c * h.to_radians().cos(), c * h.to_radians().sin());
    let l_ = (l + 0.396_337_777_4 * a + 0.215_803_757_3 * b).powi(3);
    let m_ = (l - 0.105_561_345_8 * a - 0.063_854_172_8 * b).powi(3);
    let s_ = (l - 0.089_484_177_5 * a - 1.291_485_548 * b).powi(3);
    Rgb {
        r: from_linear(4.076_741_662_1 * l_ - 3.307_711_591_3 * m_ + 0.230_969_929_2 * s_),
        g: from_linear(-1.268_438_004_6 * l_ + 2.609_757_401_1 * m_ - 0.341_319_396_5 * s_),
        b: from_linear(-0.004_196_086_3 * l_ - 0.703_418_614_7 * m_ + 1.707_614_701 * s_),
    }
}

impl Rgb {
    fn bytes(&self) -> (u8, u8, u8) {
        let q = |v: f64| (clamp01(v) * 255.0).round() as u8;
        (q(self.r), q(self.g), q(self.b))
    }

    pub fn hex(&self) -> String {
        let (r, g, b) = self.bytes();
        format!("#{r:02x}{g:02x}{b:02x}")
    }

    pub fn rgb(&self) -> String {
        let (r, g, b) = self.bytes();
        format!("rgb({r}, {g}, {b})")
    }

    pub fn hsl(&self) -> String {
        let (r, g, b) = (self.r, self.g, self.b);
        let max = r.max(g).max(b);
        let min = r.min(g).min(b);
        let l = (max + min) / 2.0;
        let d = max - min;
        let (h, s) = if d == 0.0 {
            (0.0, 0.0)
        } else {
            let s = d / (1.0 - (2.0 * l - 1.0).abs());
            let h = if max == r {
                60.0 * ((g - b) / d).rem_euclid(6.0)
            } else if max == g {
                60.0 * ((b - r) / d + 2.0)
            } else {
                60.0 * ((r - g) / d + 4.0)
            };
            (h, s)
        };
        format!("hsl({}, {}%, {}%)", h.round(), (s * 100.0).round(), (l * 100.0).round())
    }

    pub fn oklch(&self) -> String {
        let (r, g, b) = (to_linear(self.r), to_linear(self.g), to_linear(self.b));
        let l = (0.412_221_470_8 * r + 0.536_332_536_3 * g + 0.051_445_992_9 * b).cbrt();
        let m = (0.211_903_498_2 * r + 0.680_699_545_1 * g + 0.107_396_956_6 * b).cbrt();
        let s = (0.088_302_461_9 * r + 0.281_718_837_6 * g + 0.629_978_700_5 * b).cbrt();
        let ll = 0.210_454_255_3 * l + 0.793_617_785 * m - 0.004_072_046_8 * s;
        let a = 1.977_998_495_1 * l - 2.428_592_205 * m + 0.450_593_709_9 * s;
        let bb = 0.025_904_037_1 * l + 0.782_771_766_2 * m - 0.808_675_766 * s;
        let c = (a * a + bb * bb).sqrt();
        let h = if c < 1e-4 { 0.0 } else { bb.atan2(a).to_degrees().rem_euclid(360.0) };
        format!("oklch({:.1}% {:.3} {:.1})", ll * 100.0, c, h)
    }

    pub fn luminance(&self) -> f64 {
        0.2126 * to_linear(self.r) + 0.7152 * to_linear(self.g) + 0.0722 * to_linear(self.b)
    }
}

pub fn contrast(a: Rgb, b: Rgb) -> f64 {
    let (la, lb) = (a.luminance(), b.luminance());
    (la.max(lb) + 0.05) / (la.min(lb) + 0.05)
}

/// WCAG verdict for normal text, then large text.
pub fn wcag(ratio: f64) -> String {
    let level = |aa: f64, aaa: f64| {
        if ratio >= aaa {
            "AAA"
        } else if ratio >= aa {
            "AA"
        } else {
            "échec"
        }
    };
    crate::i18n::tf("texte normal {a}, grand texte {b}", &[("a", &crate::i18n::t(level(4.5, 7.0))), ("b", &crate::i18n::t(level(3.0, 4.5)))])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_and_converts() {
        let c = parse("#ff8800").unwrap();
        assert_eq!(c.rgb(), "rgb(255, 136, 0)");
        assert_eq!(c.hsl(), "hsl(32, 100%, 50%)");
        assert_eq!(parse("#f80").unwrap().hex(), "#ff8800");
        assert_eq!(parse("rgb(255 136 0)").unwrap().hex(), "#ff8800");
        assert_eq!(parse("hsl(32, 100%, 50%)").unwrap().hex(), "#ff8800");
        assert!(parse("pas une couleur").is_none());
    }

    #[test]
    fn oklch_roundtrip() {
        for hex in ["#ff8800", "#1c1a2e", "#3fb950", "#ffffff", "#000000"] {
            let c = parse(hex).unwrap();
            let back = parse(&c.oklch()).unwrap();
            assert_eq!(back.hex(), hex, "{}", c.oklch());
        }
        assert_eq!(parse("#ffffff").unwrap().oklch(), "oklch(100.0% 0.000 0.0)");
    }

    #[test]
    fn wcag_contrast() {
        let (w, k) = (parse("#fff").unwrap(), parse("#000").unwrap());
        assert!((contrast(w, k) - 21.0).abs() < 0.01);
        let r = contrast(parse("#767676").unwrap(), w);
        assert!((r - 4.54).abs() < 0.01, "{r}");
        assert_eq!(wcag(r), "texte normal AA, grand texte AAA");
        assert_eq!(wcag(2.0), "texte normal échec, grand texte échec");
    }
}
