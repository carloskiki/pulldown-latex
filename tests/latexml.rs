use std::fs::File;
use std::path::Path;

use common::{html_template, tabled, OUTPUT_DIR};

mod common;

fn main() {
    let concl = common::test();
    if std::env::var("RENDER").as_deref() != Ok("true") {
        concl.exit()
    }

    let mut file = File::create(Path::new(OUTPUT_DIR).join("latexml.html")).unwrap();
    html_template(&mut file, "LaTeXML Tests", None, tabled).unwrap();

    concl.exit();
}

round_trip_display!(
    lorenz_eqautions,
    r"
    \begin{aligned}
\dot{x} & = \sigma(y-x) \\
\dot{y} & = \rho x - y - xz \\
\dot{z} & = -\beta z + xy
\end{aligned}
    "
);

round_trip_display!(
    cauchy_shwartz_inequality,
    r"
    \left( \sum_{k=1}^n a_k b_k \right)^2
\leq \left( \sum_{k=1}^n a_k^2 \right)
\left( \sum_{k=1}^n b_k^2 \right)
    "
);

round_trip_display!(
    cross_product,
    r"
    \mathbf{V}_1 \times \mathbf{V}_2 =
\begin{vmatrix}
\mathbf{i} & \mathbf{j} & \mathbf{k} \\
\frac{\partial X}{\partial u} &
\frac{\partial Y}{\partial u} & 0 \\
\frac{\partial X}{\partial v} &
\frac{\partial Y}{\partial v} & 0
\end{vmatrix}
    "
);

round_trip_display!(
    n_choose_k,
    r"
    P(E) = \binom{n}{k} p^k (1-p)^{ n-k} \
    "
);

round_trip_display!(
    ramanujan_identity,
    r"
\frac{1}{\Bigl(\sqrt{\phi \sqrt{5}}-
\phi\Bigr) e^{\frac25 \pi}} =
1+\frac{e^{-2\pi}} {1+\frac{e^{-4\pi}}
{1+\frac{e^{-6\pi}}
{1+\frac{e^{-8\pi}} {1+\ldots} } } }
    "
);

round_trip_display!(
    rogers_ramanujan_identity,
    r"
1 + \frac{q^2}{(1-q)}+
\frac{q^6}{(1-q)(1-q^2)}+\cdots =
\prod_{j=0}^{\infty}\frac{1}
{(1-q^{5j+2})(1-q^{5j+3})},
\quad\quad \text{for} |q|<1.
    "
);

round_trip_display!(
    maxwell_equations,
    r"
\begin{aligned}
\nabla \times \vec{\mathbf{B}} -
\frac1c\, \frac{\partial\vec{
\mathbf{E}}}{\partial t} &
= \frac{4\pi}{c}\vec{\mathbf{j}} \\
\nabla \cdot \vec{\mathbf{E}} &
= 4 \pi \rho \\
\nabla \times \vec{\mathbf{E}}\, +\,
\frac1c\, \frac{\partial\vec{
\mathbf{B}}}{\partial t} &
= \vec{\mathbf{0}} \\
\nabla \cdot \vec{\mathbf{B}} &
= 0
\end{aligned}
    "
);
