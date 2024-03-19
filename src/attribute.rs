use std::fmt::Display;

pub type Dimension = (f32, DimensionUnit);
pub type Glue = (Dimension, Option<Dimension>, Option<Dimension>);

/// Fonts
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Font {
    BoldScript,
    BoldItalic,
    Bold,
    Fraktur,
    Script,
    Monospace,
    SansSerif,
    DoubleStruck,
    Italic,
    BoldFraktur,
    SansSerifBoldItalic,
    SansSerifItalic,
    BoldSansSerif,
    UpRight,
}

impl Font {
    /// Map a character to its mathvariant equivalent.
    pub fn map_char(self, c: char) -> char {
        char::from_u32(match (self, c) {
            // Bold Script mappings
            (Font::BoldScript, 'A'..='Z') => c as u32 + 0x1D48F,
            (Font::BoldScript, 'a'..='z') => c as u32 + 0x1D489,

            // Bold Italic mappings
            (Font::BoldItalic, 'A'..='Z') => c as u32 + 0x1D427,
            (Font::BoldItalic, 'a'..='z') => c as u32 + 0x1D421,
            (Font::BoldItalic, '\u{0391}'..='\u{03A1}' | '\u{03A3}'..='\u{03A9}') => {
                c as u32 + 0x1D38B
            }
            (Font::BoldItalic, '\u{03F4}') => c as u32 + 0x1D339,
            (Font::BoldItalic, '\u{2207}') => c as u32 + 0x1B52E,
            (Font::BoldItalic, '\u{03B1}'..='\u{03C9}') => c as u32 + 0x1D385,
            (Font::BoldItalic, '\u{2202}') => c as u32 + 0x1B54D,
            (Font::BoldItalic, '\u{03F5}') => c as u32 + 0x1D35B,
            (Font::BoldItalic, '\u{03D1}') => c as u32 + 0x1D380,
            (Font::BoldItalic, '\u{03F0}') => c as u32 + 0x1D362,
            (Font::BoldItalic, '\u{03D5}') => c as u32 + 0x1D37E,
            (Font::BoldItalic, '\u{03F1}') => c as u32 + 0x1D363,
            (Font::BoldItalic, '\u{03D6}') => c as u32 + 0x1D37F,

            // Bold mappings
            (Font::Bold, 'A'..='Z') => c as u32 + 0x1D3BF,
            (Font::Bold, 'a'..='z') => c as u32 + 0x1D3B9,
            (Font::Bold, '\u{0391}'..='\u{03A1}' | '\u{03A3}'..='\u{03A9}') => c as u32 + 0x1D317,
            (Font::Bold, '\u{03F4}') => c as u32 + 0x1D2C5,
            (Font::Bold, '\u{2207}') => c as u32 + 0x1B4BA,
            (Font::Bold, '\u{03B1}'..='\u{03C9}') => c as u32 + 0x1D311,
            (Font::Bold, '\u{2202}') => c as u32 + 0x1B4D9,
            (Font::Bold, '\u{03F5}') => c as u32 + 0x1D2E7,
            (Font::Bold, '\u{03D1}') => c as u32 + 0x1D30C,
            (Font::Bold, '\u{03F0}') => c as u32 + 0x1D2EE,
            (Font::Bold, '\u{03D5}') => c as u32 + 0x1D30A,
            (Font::Bold, '\u{03F1}') => c as u32 + 0x1D2EF,
            (Font::Bold, '\u{03D6}') => c as u32 + 0x1D30B,
            (Font::Bold, '\u{03DC}' | '\u{03DD}') => c as u32 + 0x1D7CA,
            (Font::Bold, '0'..='9') => c as u32 + 0x1D79E,

            // Fraktur mappings
            (Font::Fraktur, 'A' | 'B' | 'D'..='G' | 'J'..='Q' | 'S'..='Y') => c as u32 + 0x1D4C3,
            (Font::Fraktur, 'C') => c as u32 + 0x20EA,
            (Font::Fraktur, 'H' | 'I') => c as u32 + 0x20C4,
            (Font::Fraktur, 'R') => c as u32 + 0x20CA,
            (Font::Fraktur, 'Z') => c as u32 + 0x20CE,
            (Font::Fraktur, 'a'..='z') => c as u32 + 0x1D4BD,

            // Script mappings
            (Font::Script, 'A' | 'C' | 'D' | 'G' | 'J' | 'K' | 'N'..='Q' | 'S'..='Z') => {
                c as u32 + 0x1D45B
            }
            (Font::Script, 'B') => c as u32 + 0x20EA,
            (Font::Script, 'E' | 'F') => c as u32 + 0x20EB,
            (Font::Script, 'H') => c as u32 + 0x20C3,
            (Font::Script, 'I') => c as u32 + 0x20C7,
            (Font::Script, 'L') => c as u32 + 0x20C6,
            (Font::Script, 'M') => c as u32 + 0x20E6,
            (Font::Script, 'R') => c as u32 + 0x20C9,
            (Font::Script, 'a'..='d' | 'f' | 'h'..='n' | 'p'..='z') => c as u32 + 0x1D455,
            (Font::Script, 'e') => c as u32 + 0x20CA,
            (Font::Script, 'g') => c as u32 + 0x20A3,
            (Font::Script, 'o') => c as u32 + 0x20C5,

            // Monospace mappings
            (Font::Monospace, 'A'..='Z') => c as u32 + 0x1D62F,
            (Font::Monospace, 'a'..='z') => c as u32 + 0x1D629,
            (Font::Monospace, '0'..='9') => c as u32 + 0x1D7C6,

            // Sans Serif mappings
            (Font::SansSerif, 'A'..='Z') => c as u32 + 0x1D55F,
            (Font::SansSerif, 'a'..='z') => c as u32 + 0x1D559,
            (Font::SansSerif, '0'..='9') => c as u32 + 0x1D7B2,

            // Double Struck mappings
            (Font::DoubleStruck, 'A' | 'B' | 'D'..='G' | 'I'..='M' | 'O' | 'S'..='Y') => {
                c as u32 + 0x1D4F7
            }
            (Font::DoubleStruck, 'C') => c as u32 + 0x20BF,
            (Font::DoubleStruck, 'H') => c as u32 + 0x20C5,
            (Font::DoubleStruck, 'N') => c as u32 + 0x20C7,
            (Font::DoubleStruck, 'P' | 'Q') => c as u32 + 0x20C9,
            (Font::DoubleStruck, 'R') => c as u32 + 0x20CB,
            (Font::DoubleStruck, 'Z') => c as u32 + 0x20CA,
            (Font::DoubleStruck, 'a'..='z') => c as u32 + 0x1D4F1,
            (Font::DoubleStruck, '0'..='9') => c as u32 + 0x1D7A8,

            // Italic mappings
            (Font::Italic, 'A'..='Z') => c as u32 + 0x1D3F3,
            (Font::Italic, 'a'..='g' | 'i'..='z') => c as u32 + 0x1D3ED,
            (Font::Italic, 'h') => c as u32 + 0x20A6,
            (Font::Italic, '\u{0391}'..='\u{03A1}' | '\u{03A3}'..='\u{03A9}') => c as u32 + 0x1D351,
            (Font::Italic, '\u{03F4}') => c as u32 + 0x1D2FF,
            (Font::Italic, '\u{2207}') => c as u32 + 0x1B4F4,
            (Font::Italic, '\u{03B1}'..='\u{03C9}') => c as u32 + 0x1D34B,
            (Font::Italic, '\u{2202}') => c as u32 + 0x1B513,
            (Font::Italic, '\u{03F5}') => c as u32 + 0x1D321,
            (Font::Italic, '\u{03D1}') => c as u32 + 0x1D346,
            (Font::Italic, '\u{03F0}') => c as u32 + 0x1D328,
            (Font::Italic, '\u{03D5}') => c as u32 + 0x1D344,
            (Font::Italic, '\u{03F1}') => c as u32 + 0x1D329,
            (Font::Italic, '\u{03D6}') => c as u32 + 0x1D345,

            // Bold Fraktur mappings
            (Font::BoldFraktur, 'A'..='Z') => c as u32 + 0x1D52B,
            (Font::BoldFraktur, 'a'..='z') => c as u32 + 0x1D525,

            // Sans Serif Bold Italic mappings
            (Font::SansSerifBoldItalic, 'A'..='Z') => c as u32 + 0x1D5FB,
            (Font::SansSerifBoldItalic, 'a'..='z') => c as u32 + 0x1D5F5,
            (Font::SansSerifBoldItalic, '\u{0391}'..='\u{03A1}' | '\u{03A3}'..='\u{03A9}') => {
                c as u32 + 0x1D3FF
            }
            (Font::SansSerifBoldItalic, '\u{03F4}') => c as u32 + 0x1D3AD,
            (Font::SansSerifBoldItalic, '\u{2207}') => c as u32 + 0x1B5A2,
            (Font::SansSerifBoldItalic, '\u{03B1}'..='\u{03C9}') => c as u32 + 0x1D3F9,
            (Font::SansSerifBoldItalic, '\u{2202}') => c as u32 + 0x1B5C1,
            (Font::SansSerifBoldItalic, '\u{03F5}') => c as u32 + 0x1D3CF,
            (Font::SansSerifBoldItalic, '\u{03D1}') => c as u32 + 0x1D3F4,
            (Font::SansSerifBoldItalic, '\u{03F0}') => c as u32 + 0x1D3D6,
            (Font::SansSerifBoldItalic, '\u{03D5}') => c as u32 + 0x1D3F2,
            (Font::SansSerifBoldItalic, '\u{03F1}') => c as u32 + 0x1D3D7,
            (Font::SansSerifBoldItalic, '\u{03D6}') => c as u32 + 0x1D3F3,

            // Sans Serif Italic mappings
            (Font::SansSerifItalic, 'A'..='Z') => c as u32 + 0x1D5D7,
            (Font::SansSerifItalic, 'a'..='z') => c as u32 + 0x1D5C1,

            // Bold Sans Serif mappings
            (Font::BoldSansSerif, 'A'..='Z') => c as u32 + 0x1D593,
            (Font::BoldSansSerif, 'a'..='z') => c as u32 + 0x1D58D,
            (Font::BoldSansSerif, '\u{0391}'..='\u{03A1}' | '\u{03A3}'..='\u{03A9}') => {
                c as u32 + 0x1D3C5
            }
            (Font::BoldSansSerif, '\u{03F4}') => c as u32 + 0x1D373,
            (Font::BoldSansSerif, '\u{2207}') => c as u32 + 0x1B568,
            (Font::BoldSansSerif, '\u{03B1}'..='\u{03C9}') => c as u32 + 0x1D3BF,
            (Font::BoldSansSerif, '\u{2202}') => c as u32 + 0x1B587,
            (Font::BoldSansSerif, '\u{03F5}') => c as u32 + 0x1D395,
            (Font::BoldSansSerif, '\u{03D1}') => c as u32 + 0x1D3BA,
            (Font::BoldSansSerif, '\u{03F0}') => c as u32 + 0x1D39C,
            (Font::BoldSansSerif, '\u{03D5}') => c as u32 + 0x1D3B8,
            (Font::BoldSansSerif, '\u{03F1}') => c as u32 + 0x1D39D,
            (Font::BoldSansSerif, '\u{03D6}') => c as u32 + 0x1D3B9,
            (Font::BoldSansSerif, '0'..='9') => c as u32 + 0x1D7BC,

            // Upright mappings (map to themselves) and unknown mappings
            (_, _) => c as u32,
        })
        .expect("character not in Unicode (developer error)")
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DimensionUnit {
    Em,
    Ex,
    Pt,
    Pc,
    In,
    Bp,
    Cm,
    Mm,
    Dd,
    Cc,
    Sp,
    Mu,
}

/// Convert TeX units to CSS units.
///
/// This is similar to the conversion table in the TeXbook p. 57, but is not exact.
///
/// The use of `em` and `mu` units are recommended, as they match exactly their definition. Using
/// other units may result in unexpected sizing, as all dimensions are converted to `em` units when
/// rendering.
///
/// This function uses the following conversion table:
/// | TeX unit | css _em_ |
/// | -------- | -------- |
/// | pt       | 0.1      |
/// | pc       | 1.2      |
/// | bp       | 0.0996   |
/// | dd       | 0.107    |
/// | cc       | 1.07     |
/// | sp       | 1.5e-6   |
/// | mu       | 1/18     |
/// | em       | 1        |
/// | ex       | 0.4      |
/// | in       | 6        |
/// | cm       | 6/2.54   |
/// | mm       | 6/25.4   |
pub fn tex_to_css_em(dim: Dimension) -> f32 {
    match dim.1 {
        DimensionUnit::Pt => dim.0 * 0.1,
        DimensionUnit::Pc => dim.0 * 1.2,
        DimensionUnit::Bp => dim.0 * 72. / 72.27 * 0.1,
        DimensionUnit::Dd => dim.0 * 1238. / 1157. * 0.1,
        DimensionUnit::Cc => dim.0 * 12. * 1238. / 1157. * 0.1,
        DimensionUnit::Sp => dim.0 / 65536. * 0.1,
        DimensionUnit::Mu => dim.0 / 18.,
        DimensionUnit::Em => dim.0,
        DimensionUnit::Ex => dim.0 * 0.4,
        DimensionUnit::In => dim.0 * 6.,
        DimensionUnit::Cm => dim.0 * 6. / 2.54,
        DimensionUnit::Mm => dim.0 * 6. / 25.4,
    }
}
