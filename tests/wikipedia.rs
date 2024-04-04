mod common;

common::round_trip!(
    accents,
    r"
\dot{a}, \ddot{a}, \acute{a}, \grave{a}
\check{a}, \breve{a}, \tilde{a}, \bar{a}	
\hat{a}, \widehat{a}, \vec{a}	
    "
);

common::round_trip!(
    greek_uppercase,
    r"
\Alpha \Beta \Gamma \Delta \Epsilon \Zeta \Eta \Theta
\Iota \Kappa \Lambda \Mu \Nu \Xi \Omicron \Pi
\Rho \Sigma \Tau \Upsilon \Phi \Chi \Psi \Omega
        "
);

common::round_trip!(
    greek_lowercase,
    r"
\alpha \beta \gamma \delta \epsilon \zeta \eta \theta
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

common::round_trip!(
    standard_numerical_functions,
    r"
\exp_a b = a^b, \exp b = e^b, 10^m
\ln c = \log c, \lg d = \log_{10} d	
\sin a, \cos b, \tan c, \cot d, \sec f, \csc g	
\arcsin h, \arccos i, \arctan j	
\sinh k, \cosh l, \tanh m, \coth n	
\operatorname{sh}k, \operatorname{ch}l, \operatorname{th}m, \operatorname{coth}n
\operatorname{argsh}o, \operatorname{argch}p, \operatorname{argth}q	
\sgn r, \left\vert s \right\vert	
\min(x,y), \max(x,y)	
    "
);

common::round_trip!(
    bounds,
    r"
\min x, \max y, \inf s, \sup t	
\lim u, \liminf v, \limsup w	
\dim p, \deg q, \det m, \ker\phi	
    "
);

common::round_trip!(
    projections,
    r"
\Pr j, \hom l, \lVert z \rVert, \arg z	
    "
);

common::round_trip!(
    differential_and_derivatives,
    r"
dt, \mathrm{d}t, \partial t, \nabla\psi	
dy/dx, \mathrm{d}y/\mathrm{d}x, \frac{dy}{dx}, \frac{\mathrm{d}y}{\mathrm{d}x}	
\frac{\partial^2}{\partial x_1\partial x_2}y, \left.\frac{\partial^3 f}{\partial^2 x \partial y}\right\vert_{p_0}	
\prime, \backprime, f^\prime, f', f'', f^{(3)}, \dot y, \ddot y	
    "
);



common::round_trip!(
    letter_like_symbols_or_constants,
    r"
\infty, \aleph, \complement, \backepsilon, \eth, \Finv, \hbar, \N, \R, \Z, \C, \Q	
\Im, \imath, \jmath, \Bbbk, \ell, \mho, \wp, \Re, \circledS, \S, \P, \AA	
    "
);

common::round_trip!(
    modular_arithmetic,
    r"
s_k \equiv 0 \pmod{m}	
a \bmod b	
\gcd(m, n), \operatorname{lcm}(m, n)	
\mid, \nmid, \shortmid, \nshortmid	
    "
);

common::round_trip!(
    radicals,
    r"\surd, \sqrt{2}, \sqrt[n]{2}, \sqrt[3]{\frac{x^3+y^3}{2}}"
);

common::round_trip!(
    operators,
    r"
+, -, \pm, \mp, \dotplus	
\times, \div, \divideontimes, /, \backslash	
\cdot, * \ast, \star, \circ, \bullet	
\boxplus, \boxminus, \boxtimes, \boxdot	
\oplus, \ominus, \otimes, \oslash, \odot	
\circleddash, \circledcirc, \circledast	
\bigoplus, \bigotimes, \bigodot	
    "
);

common::round_trip!(
    sets,
    r"
\{ \}, \O \empty \emptyset, \varnothing	
\in, \notin \not\in, \ni, \not\ni	
\cap, \Cap, \sqcap, \bigcap	
\cup, \Cup, \sqcup, \bigcup, \bigsqcup, \uplus, \biguplus	
\setminus, \smallsetminus, \times	
\subset, \Subset, \sqsubset	
\supset, \Supset, \sqsupset	
\subseteq, \nsubseteq, \subsetneq, \varsubsetneq, \sqsubseteq	
\supseteq, \nsupseteq, \supsetneq, \varsupsetneq, \sqsupseteq	
\subseteqq, \nsubseteqq, \subsetneqq, \varsubsetneqq	
\supseteqq, \nsupseteqq, \supsetneqq, \varsupsetneqq	
"
);

common::round_trip!(
    relations,
    r"
=, \ne, \neq, \equiv, \not\equiv	
\doteq, \doteqdot, \overset{\underset{\mathrm{def}}{}}{=}, :=	
\sim, \nsim, \backsim, \thicksim, \simeq, \backsimeq, \eqsim, \cong, \ncong	
\approx, \thickapprox, \approxeq, \asymp, \propto, \varpropto	
<, \nless, \ll, \not\ll, \lll, \not\lll, \lessdot	
>, \ngtr, \gg, \not\gg, \ggg, \not\ggg, \gtrdot	
\le, \leq, \lneq, \leqq, \nleq, \nleqq, \lneqq, \lvertneqq	
\ge, \geq, \gneq, \geqq, \ngeq, \ngeqq, \gneqq, \gvertneqq	
\lessgtr, \lesseqgtr, \lesseqqgtr, \gtrless, \gtreqless, \gtreqqless	
\leqslant, \nleqslant, \eqslantless	
\geqslant, \ngeqslant, \eqslantgtr	
\lesssim, \lnsim, \lessapprox, \lnapprox	
\gtrsim, \gnsim, \gtrapprox, \gnapprox	
\prec, \nprec, \preceq, \npreceq, \precneqq	
\succ, \nsucc, \succeq, \nsucceq, \succneqq	
\preccurlyeq, \curlyeqprec	
\succcurlyeq, \curlyeqsucc	
\precsim, \precnsim, \precapprox, \precnapprox	
\succsim, \succnsim, \succapprox, \succnapprox	
"
);

common::round_trip!(
    geometric,
    r"
\parallel, \nparallel, \shortparallel, \nshortparallel	
\perp, \angle, \sphericalangle, \measuredangle, 45^\circ	
\Box, \square, \blacksquare, \diamond, \Diamond, \lozenge, \blacklozenge, \bigstar	
\bigcirc, \triangle, \bigtriangleup, \bigtriangledown	
\vartriangle, \triangledown	
\blacktriangle, \blacktriangledown, \blacktriangleleft, \blacktriangleright	
    "
);

common::round_trip!(
    logic,
    r"
\forall, \exists, \nexists	
\therefore, \because, \And	
\lor, \vee, \curlyvee, \bigvee
\land, \wedge, \curlywedge, \bigwedge
\bar{q}, \bar{abc}, \overline{q}, \overline{abc},
\lnot, \neg, \not\operatorname{R}, \bot, \top
\vdash, \dashv, \vDash, \Vdash, \models	
\Vvdash, \nvdash, \nVdash, \nvDash, \nVDash	
\ulcorner, \urcorner, \llcorner, \lrcorner	
    "
);
