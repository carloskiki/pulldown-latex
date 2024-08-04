use criterion::{criterion_group, criterion_main, Criterion};
use pulldown_latex::parser::Parser;

fn match_on_greek(c: &mut Criterion) {
    c.bench_function("match on greek", |b| b.iter(|| {
    let storage = pulldown_latex::Storage::new();
    let parser = Parser::new(r"
\alpha \beta \gamma \delta \epsilon \zeta \eta \theta
\iota \kappa \lambda \mu \nu \xi \omicron \pi
\rho \sigma \tau \upsilon \phi \chi \psi \omega
\Alpha \Beta \Gamma \Delta \Epsilon \Zeta \Eta \Theta
\Iota \Kappa \Lambda \Mu \Nu \Xi \Omicron \Pi
\Rho \Sigma \Tau \Upsilon \Phi \Chi \Psi \Omega
\varGamma \varDelta \varTheta \varLambda \varXi \varPi \varSigma \varUpsilon \varPhi \varPsi \varOmega
\varepsilon \vartheta \varkappa \varrho \varsigma \varpi \digamma \varphi
\alpha \beta \gamma \delta \epsilon \zeta \eta \theta
\alpha \beta \gamma \delta \epsilon \zeta \eta \theta
\iota \kappa \lambda \mu \nu \xi \omicron \pi
\rho \sigma \tau \upsilon \phi \chi \psi \omega
\Alpha \Beta \Gamma \Delta \Epsilon \Zeta \Eta \Theta
\Iota \Kappa \Lambda \Mu \Nu \Xi \Omicron \Pi
\Rho \Sigma \Tau \Upsilon \Phi \Chi \Psi \Omega
\varGamma \varDelta \varTheta \varLambda \varXi \varPi \varSigma \varUpsilon \varPhi \varPsi \varOmega
\varepsilon \vartheta \varkappa \varrho \varsigma \varpi \digamma \varphi
\iota \kappa \lambda \mu \nu \xi \omicron \pi
\rho \sigma \tau \upsilon \phi \chi \psi \omega
\Alpha \Beta \Gamma \Delta \Epsilon \Zeta \Eta \Theta
\Iota \Kappa \Lambda \Mu \Nu \Xi \Omicron \Pi
", &storage);
        let mut str = String::new();
        pulldown_latex::mathml::push_mathml(&mut str, parser, Default::default()).unwrap();
    }));
}

fn subscript_torture(c: &mut Criterion) {
    c.bench_function("subscript torture", |b| {
        b.iter(|| {
            let storage = pulldown_latex::Storage::new();
            let parser = Parser::new("a_{5_{5_{5_{5_{5_{5_{5_{5_{5_{5_{5_5}}}}}}}}}}}", &storage);
            let mut str = String::new();
            pulldown_latex::mathml::push_mathml(&mut str, parser, Default::default()).unwrap();
        })
    });
}

fn basic_macro(c: &mut Criterion) {
    c.bench_function("basic macro", |b| {
        b.iter(|| {
            let storage = pulldown_latex::Storage::new();
            let parser = Parser::new(
                r"\def\d{\mathrm{d}}
                \oint_C \vec{B}\circ \d\vec{l} = \mu_0 \left( I_{\text{enc}}
                + \varepsilon_0 \frac{\d}{\d t} \int_S {\vec{E} \circ \hat{n}}\;
                \d a \right)",
                &storage,
            );
            let mut str = String::new();
            pulldown_latex::mathml::push_mathml(&mut str, parser, Default::default()).unwrap();
        })
    });
}

criterion_group!(benches, basic_macro, match_on_greek, subscript_torture);
criterion_main!(benches);
