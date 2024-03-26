mod common;

common::round_trip!(
    greek_uppercase,
    r"\Alpha \Beta \Gamma \Delta \Epsilon \Zeta \Eta \Theta
        \Iota \Kappa \Lambda \Mu \Nu \Xi \Omicron \Pi
        \Rho \Sigma \Tau \Upsilon \Phi \Chi \Psi \Omega
        "
);

common::round_trip!(
    greek_lowercase,
    r"\alpha \beta \gamma \delta \epsilon \zeta \eta \theta
        \iota \kappa \lambda \mu \nu \xi \omicron \pi
        \rho \sigma \tau \upsilon \phi \chi \psi \omega
        "
);

common::round_trip!(
    greek_uppercase_variants,
    r"\varGamma \varDelta \varTheta \varLambda \varXi \varPi \varSigma \varUpsilon \varPhi \varPsi \varOmega"
);

common::round_trip!(
    greek_lowercase_variants,
    r"\varepsilon \vartheta \varkappa \varrho \varsigma \varpi \digamma \varphi"
);
