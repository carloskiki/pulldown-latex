use std::fs::File;
use std::path::Path;

use common::{tabled, OUTPUT_DIR, html_template};
use pulldown_latex::config::DisplayMode;

mod common;

fn main() {
    let concl = common::test();
    if std::env::var("RENDER").as_deref() != Ok("true") {
        concl.exit()
    }

    let mut file = File::create(Path::new(OUTPUT_DIR).join("mozilla.html")).unwrap();
    html_template(&mut file, "Mozilla Tests", None, tabled).unwrap();

    concl.exit();
}

round_trip!(
    scripts,
    "x^2y^2",
    "_2F_3",
    "x^{2y}",
    "2^{2^{2^x}}",
    "y_{x^2}",
    "y_{x_2}",
    r"x_{92}^{31415} + \pi",
    "x_{y^a_b}^{z^c_d}",
    "y_3'''",
    display_mode = DisplayMode::Block
);

round_trip!(
    fractions,
    r"\frac{x+y^2}{k+1}",
    r"x+y^\frac{2}{k + 1}",
    r"\frac{a}{b/2}",
    r"a_0 + \cfrac{1}{a_1 + \cfrac{1}{a_2 + \cfrac{1}{a_3 + \cfrac{1}{a_4}}}}",
    r"a_0+\frac{1}{a_1+\frac{1}{a_2+\frac{1}{a_3+ \frac{1}{a_4}}}}",
    r"\binom{p}{2} x^2 y^{p-2} - \frac{1}{1-x} \frac{1}{1-x^2}",
    display_mode = DisplayMode::Block
);

round_trip!(
    summations,
    r"\sum_{\genfrac{}{}{0mu}{2}{0 \le i \le m}{0 < j < n}} P(i, j)",
    r"\sum_{i=1}^p \sum_{j=1}^q \sum_{k=1}^r a_{ij}b_{jk}c_{ki}",
    display_mode = DisplayMode::Block
);

round_trip!(
    roots,
    r"\sqrt{1+\sqrt{1+\sqrt{1+ \sqrt{1+\sqrt{1+\sqrt{1+ \sqrt{1+x}}}}}}}",
    display_mode = DisplayMode::Block
);

round_trip!(
    differentials,
    r"\bigg(\frac{\partial^2} {\partial x^2} + \frac {\partial^2}{\partial y^2} \bigg){\big\lvert\varphi (x+iy)\big\rvert}^2",
    display_mode = DisplayMode::Block
);

round_trip!(
    integrals,
    r"\int_1^x \frac{dt}{t}",
    r"\int\!\!\!\int_D dx,dy",
    display_mode = DisplayMode::Block
);

round_trip!(
    environments,
    r"f(x) = \begin{cases}1/3 & \text{if }0 \le x \le 1; \\ 2/3 & \text{if }3\le x \le 4;\\ 0 &\text{elsewhere.} \end{cases}",
    r"\begin{pmatrix}
        \begin{pmatrix}a&b\\c&d
        \end{pmatrix} &
        \begin{pmatrix}e&f\\g&h
        \end{pmatrix} \\ 0 &
        \begin{pmatrix}i&j\\k&l
        \end{pmatrix}
        \end{pmatrix}",
    r"\det\begin{vmatrix}
        c_0&c_1&c_2&\dots& c_n\\
        c_1 & c_2 & c_3 & \dots &
        c_{n+1}\\ c_2 & c_3 & c_4
        &\dots & c_{n+2}\\ \vdots
        &\vdots&\vdots & &\vdots
        \\c_n & c_{n+1} & c_{n+2}
        &\dots&c_{2n}
        \end{vmatrix} > 0",
 display_mode = DisplayMode::Block
);

round_trip!(
    over_under_braces,
    r"\overbrace{x +\cdots + x} ^{k \text{ times}}",
    r"{\underbrace{\overbrace{ \mathstrut a,\dots,a}^{k ,a\rq\text{s}}, \overbrace{ \mathstrut b,\dots,b}^{l, b\rq\text{s}}}_{k+l \text{ elements}}}",
    display_mode = DisplayMode::Block
);

round_trip!(
    everything,
    r"\sum_{p\text{ prime}} f(p)=\int_{t>1} f(t)d\pi(t)",
    r"\lim_{n \to +\infty} \frac{\sqrt{2\pi n}}{n!} \genfrac (){}{}n{e}^n = 1",
    r"\det(A) = \sum_{\sigma \in S_n} \epsilon(\sigma) \prod_{i=1}^n a_{i, \sigma_i}",
    display_mode = DisplayMode::Block
);
