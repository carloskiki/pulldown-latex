// TODO: Consider using `phf`s for this

// Faster impl, but does not include all operators
// #[rustfmt::skip]
// static OPERATOR_TABLE: &[(u16, u16)] = &[
//     (33, 34), (37, 47), (58, 59), (63, 64), (91, 96),
//     (123, 126), (168, 168), (172, 172), (175, 180), (183, 185),
//     (215, 215), (247, 247), (710, 711), (713, 715), (717, 717),
//     (728, 730), (732, 733), (759, 759), (770, 770), (785, 785),
//     (800, 800), (802, 803), (805, 805), (807, 807), (814, 814),
//     (817, 817), (8214, 8214), (8216, 8223), (8226, 8226), (8242, 8247),
//     (8254, 8254), (8259, 8260), (8279, 8279), (8289, 8292), (8411, 8412),
//     (8517, 8518), (8592, 8597), (8602, 8622), (8624, 8629), (8633, 8633),
//     (8636, 8661), (8666, 8688), (8691, 8708), (8710, 8711), (8719, 8732),
//     (8735, 8738), (8743, 8758), (8760, 8760), (8764, 8764), (8768, 8768),
//     (8844, 8846), (8851, 8859), (8861, 8865), (8890, 8903), (8905, 8908),
//     (8910, 8911), (8914, 8915), (8965, 8966), (8968, 8971), (8976, 8976),
//     (8985, 8985), (8994, 8995), (9001, 9002), (9140, 9141), (9165, 9165),
//     (9180, 9185), (10098, 10099), (10132, 10135), (10137, 10137), (10139, 10145),
//     (10149, 10150), (10152, 10159), (10161, 10161), (10163, 10163), (10165, 10165),
//     (10168, 10168), (10170, 10174), (10176, 10176), (10187, 10187), (10189, 10189),
//     (10214, 10225), (10228, 10239), (10496, 10528), (10548, 10551), (10562, 10613),
//     (10620, 10624), (10627, 10649), (10651, 10671), (10680, 10680), (10684, 10684),
//     (10692, 10696), (10708, 10715), (10722, 10722), (10741, 10749), (10752, 10853),
//     (10971, 10973), (10988, 10989), (10998, 10998), (11003, 11007), (11012, 11015),
//     (11020, 11025), (11056, 11070), (11072, 11084), (11104, 11109), (11114, 11117),
//     (11120, 11123), (11130, 11133), (11136, 11143), (11157, 11157), (11168, 11183),
//     (11192, 11192)
// ];

// pub fn is_operator(c: char) -> bool {
//     let c = c as u16;
//
//     let mut left = 0;
//     let mut size = OPERATOR_TABLE.len();
//     let mut right = size;
//     while left < right {
//         let mid = left + size / 2;
//         let (start, end) = OPERATOR_TABLE[mid];
//
//         if c >= start && c <= end {
//             return true;
//         }
//
//         if c < start {
//             right = mid;
//         } else if c > end {
//             left = mid + 1;
//         } else {
//             return true;
//         }
//
//         size = right - left;
//     }
//     false
// }

use super::Token;

pub fn is_operator(c: char) -> bool {
    matches!(
    c,
    '\u{0021}'..='\u{0022}' | '\u{0025}'..='\u{002F}' | '\u{003A}'..='\u{0040}' | '\u{005B}'..='\u{0060}' |
    '\u{007B}'..='\u{007E}' | '\u{00A8}' | '\u{00AC}' | '\u{00AF}'..='\u{00B4}' | '\u{00B7}'..='\u{00B9}' |
    '\u{00D7}' | '\u{00F7}' | '\u{02C6}'..='\u{02C7}' | '\u{02C9}'..='\u{02CB}' | '\u{02CD}' | '\u{02D8}'..='\u{02DA}' |
    '\u{02DC}'..='\u{02DD}' | '\u{02F7}' | '\u{0302}' | '\u{0311}' | '\u{0320}' | '\u{0322}'..='\u{0323}' |
    '\u{0325}' | '\u{0327}' | '\u{032E}' | '\u{0331}' | '\u{2016}' | '\u{2018}'..='\u{201F}' | '\u{2022}' |
    '\u{2032}'..='\u{2037}' | '\u{203E}' | '\u{2043}'..='\u{2044}' | '\u{2057}' | '\u{2061}'..='\u{2064}' |
    '\u{20DB}'..='\u{20DC}' | '\u{2145}'..='\u{2146}' | '\u{2190}'..='\u{2204}' | '\u{2206}'..='\u{220D}' |
    '\u{220F}'..='\u{221D}' | '\u{221F}'..='\u{223E}' | '\u{2240}'..='\u{22A3}' | '\u{22A6}'..='\u{22B8}' |
    '\u{22BA}'..='\u{22ED}' | '\u{22F2}'..='\u{22FF}' | '\u{2301}' | '\u{2305}'..='\u{2306}' | '\u{2308}'..='\u{230B}' |
    '\u{2310}' | '\u{2319}' | '\u{2322}'..='\u{2323}' | '\u{2329}'..='\u{232A}' | '\u{237C}' | '\u{238B}' |
    '\u{23B4}'..='\u{23B5}' | '\u{23CD}' | '\u{23DC}'..='\u{23E1}' | '\u{2772}'..='\u{2773}' | '\u{2794}'..='\u{27A1}' |
    '\u{27A5}'..='\u{27AF}' | '\u{27B1}'..='\u{27BE}' | '\u{27C0}' | '\u{27C2}' | '\u{27CB}' | '\u{27CD}' |
    '\u{27E6}'..='\u{27FF}' | '\u{2900}'..='\u{2999}' | '\u{299B}'..='\u{29AF}' | '\u{29B6}'..='\u{29B9}' |
    '\u{29BC}' | '\u{29C0}'..='\u{29C1}' | '\u{29C4}'..='\u{29C8}' | '\u{29CE}'..='\u{29DB}' | '\u{29DF}' |
    '\u{29E1}'..='\u{29E6}' | '\u{29F4}'..='\u{29FD}' | '\u{2A00}'..='\u{2AEE}' | '\u{2AF2}'..='\u{2B11}' |
    '\u{2B30}'..='\u{2B4F}' | '\u{2B5A}'..='\u{2B73}' | '\u{2B76}'..='\u{2B7D}' | '\u{2B80}'..='\u{2B8F}' |
    '\u{2B94}' | '\u{2B95}' | '\u{2BA0}'..='\u{2BB8}' | '\u{2BD1}'
    )
}

#[rustfmt::skip]
pub fn is_char_delimiter(c: char) -> bool {
    matches!(
        c,
          '(' | ')' | '⦇' | '⦈' | '⟮' | '⟯'
        | '[' | ']' | '⟦' | '⟧' | '⦃' | '⦄'
        | '⟨' | '⟩' | '⟪' | '⟫' | '⦉' | '⦊'
        | '⌊' | '⌋' | '⌈' | '⌉' | '┌' | '┐'
        | '└' | '┘' | '⎰' | '⎱' | '|' | '‖'
        | '↑' | '⇑' | '↓' | '⇓' | '↕' | '⇕'
        | '/'
    )
}

/// Returns the matching delimiter for the given control sequence, if it exists.
pub fn control_sequence_delimiter_map(cs: &str) -> Option<char> {
    Some(match cs {
        "lparen" => '(',
        "rparen" => ')',
        "llparenthesis" => '⦇',
        "rrparenthesis" => '⦈',
        "lgroup" => '⟮',
        "rgroup" => '⟯',

        "lbrack" => '[',
        "rbrack" => ']',
        "lBrack" => '⟦',
        "rBrack" => '⟧',

        "{" | "lbrace" => '{',
        "}" | "rbrace" => '}',
        "lBrace" => '⦃',
        "rBrace" => '⦄',

        "langle" => '⟨',
        "rangle" => '⟩',
        "lAngle" => '⟪',
        "rAngle" => '⟫',
        "llangle" => '⦉',
        "rrangle" => '⦊',

        "lfloor" => '⌊',
        "rfloor" => '⌋',
        "lceil" => '⌈',
        "rceil" => '⌉',
        "ulcorner" => '┌',
        "urcorner" => '┐',
        "llcorner" => '└',
        "lrcorner" => '┘',

        "lmoustache" => '⎰',
        "rmoustache" => '⎱',
        "backslash" => '\\',

        "vert" | "lvert" | "rvert" => '|',
        "|" | "Vert" | "lVert" | "rVert" => '‖',
        "uparrow" => '↑',
        "Uparrow" => '⇑',
        "downarrow" => '↓',
        "Downarrow" => '⇓',
        "updownarrow" => '↕',
        "Updownarrow" => '⇕',
        _ => return None,
    })
}

pub fn token_to_delim(token: Token) -> Option<char> {
    match token {
        Token::ControlSequence(cs) => control_sequence_delimiter_map(cs),
        Token::Character(c) if is_char_delimiter(c.into()) => Some(c.into()),
        _ => None,
    }
}

/// Returns whether the given string is a valid primitive color.
///
/// Named colors come from the [MDN docs](https://developer.mozilla.org/en-US/docs/Web/CSS/named-color#value),
/// which is a list of about 150 official css color names.
pub fn is_primitive_color(color: &str) -> bool {
    color.starts_with('#')
        && color.len() == 7
        && color.as_bytes()[1..].iter().all(|&c| c.is_ascii_hexdigit())
        || matches!(
            color,
            "aliceblue"
                | "antiquewhite"
                | "aqua"
                | "aquamarine"
                | "azure"
                | "beige"
                | "bisque"
                | "black"
                | "blanchedalmond"
                | "blue"
                | "blueviolet"
                | "brown"
                | "burlywood"
                | "cadetblue"
                | "chartreuse"
                | "chocolate"
                | "coral"
                | "cornflowerblue"
                | "cornsilk"
                | "crimson"
                | "cyan"
                | "darkblue"
                | "darkcyan"
                | "darkgoldenrod"
                | "darkgray"
                | "darkgreen"
                | "darkgrey"
                | "darkkhaki"
                | "darkmagenta"
                | "darkolivegreen"
                | "darkorange"
                | "darkorchid"
                | "darkred"
                | "darksalmon"
                | "darkseagreen"
                | "darkslateblue"
                | "darkslategray"
                | "darkslategrey"
                | "darkturquoise"
                | "darkviolet"
                | "deeppink"
                | "deepskyblue"
                | "dimgray"
                | "dimgrey"
                | "dodgerblue"
                | "firebrick"
                | "floralwhite"
                | "forestgreen"
                | "fuchsia"
                | "gainsboro"
                | "ghostwhite"
                | "gold"
                | "goldenrod"
                | "gray"
                | "green"
                | "greenyellow"
                | "grey"
                | "honeydew"
                | "hotpink"
                | "indianred"
                | "indigo"
                | "ivory"
                | "khaki"
                | "lavender"
                | "lavenderblush"
                | "lawngreen"
                | "lemonchiffon"
                | "lightblue"
                | "lightcoral"
                | "lightcyan"
                | "lightgoldenrodyellow"
                | "lightgray"
                | "lightgreen"
                | "lightgrey"
                | "lightpink"
                | "lightsalmon"
                | "lightseagreen"
                | "lightskyblue"
                | "lightslategray"
                | "lightslategrey"
                | "lightsteelblue"
                | "lightyellow"
                | "lime"
                | "limegreen"
                | "linen"
                | "magenta"
                | "maroon"
                | "mediumaquamarine"
                | "mediumblue"
                | "mediumorchid"
                | "mediumpurple"
                | "mediumseagreen"
                | "mediumslateblue"
                | "mediumspringgreen"
                | "mediumturquoise"
                | "mediumvioletred"
                | "midnightblue"
                | "mintcream"
                | "mistyrose"
                | "moccasin"
                | "navajowhite"
                | "navy"
                | "oldlace"
                | "olive"
                | "olivedrab"
                | "orange"
                | "orangered"
                | "orchid"
                | "palegoldenrod"
                | "palegreen"
                | "paleturquoise"
                | "palevioletred"
                | "papayawhip"
                | "peachpuff"
                | "peru"
                | "pink"
                | "plum"
                | "powderblue"
                | "purple"
                | "rebeccapurple"
                | "red"
                | "rosybrown"
                | "royalblue"
                | "saddlebrown"
                | "salmon"
                | "sandybrown"
                | "seagreen"
                | "seashell"
                | "sienna"
                | "silver"
                | "skyblue"
                | "slateblue"
                | "slategray"
                | "slategrey"
                | "snow"
                | "springgreen"
                | "steelblue"
                | "tan"
                | "teal"
                | "thistle"
                | "tomato"
                | "transparent"
                | "turquoise"
                | "violet"
                | "wheat"
                | "white"
                | "whitesmoke"
                | "yellow"
                | "yellowgreen"
        )
}
