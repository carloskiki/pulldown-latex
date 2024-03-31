#![feature(test)]

extern crate test;

use test::Bencher;

use pulldown_latex::parser::Parser;

#[bench]
fn match_on_greek(b: &mut Bencher) {
    b.iter(|| {
    let mut parser = test::black_box(Parser::new(r"
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
"));
        parser.map(|x| x.unwrap()).for_each(|v| {
            test::black_box(v);
        });
    });
}
