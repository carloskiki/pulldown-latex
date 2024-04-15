mod common;


fn main() {
    common::test()
}

round_trip!(
    basic,
    r"\alpha",
    r"f(x) = x^2",
    r"\{1,e,\pi\}",
    r"|z| \leq 2"
);

round_trip!(
    accents_and_diacritics,
    r"\dot{a}, \ddot{a}, \acute{a}, \grave{a}",
    r"\check{a}, \breve{a}, \tilde{a}, \bar{a}",
    r"\hat{a}, \widehat{a}, \vec{a}"
);

round_trip!(
    standard_numerical_functions,
    r"\exp_a b = a^b, \exp b = e^b, 10^m",
    r"\ln c = \log c, \lg d = \log_{10} d",
    r"\sin a, \cos b, \tan c, \cot d, \sec f, \csc g",
    r"\arcsin h, \arccos i, \arctan j",
    r"\sinh k, \cosh l, \tanh m, \coth n",
    r"\operatorname{sh}k, \operatorname{ch}l, \operatorname{th}m, \operatorname{coth}n",
    r"\operatorname{argsh}o, \operatorname{argch}p, \operatorname{argth}q",
    r"\sgn r, \left\vert s \right\vert",
    r"\min(x,y), \max(x,y)"
);

round_trip!(
    bounds,
    r"\min x, \max y, \inf s, \sup t",
    r"\lim u, \liminf v, \limsup w",
    r"\dim p, \deg q, \det m, \ker\phi"
);

round_trip!(projections, r"\Pr j, \hom l, \lVert z \rVert, \arg z");

round_trip!(
    differential_and_derivatives,
    r"dt, \mathrm{d}t, \partial t, \nabla\psi",
    r"dy/dx, \mathrm{d}y/\mathrm{d}x, \frac{dy}{dx}, \frac{\mathrm{d}y}{\mathrm{d}x}",
    r"\frac{\partial^2}{\partial x_1\partial x_2}y, \left.\frac{\partial^3 f}{\partial^2 x \partial y}\right\vert_{p_0}",
    r"\prime, \backprime, f^\prime, f', f'', f^{(3)}, \dot y, \ddot y"
);
round_trip!(
    letter_like_symbols_or_constants,
    r"\infty, \aleph, \complement, \backepsilon, \eth, \Finv, \hbar",
    r"\Im, \imath, \jmath, \Bbbk, \ell, \mho, \wp, \Re, \circledS, \S, \P"
);

round_trip!(
    modular_arithmetic,
    r"s_k \equiv 0 \pmod{m}",
    r"a \bmod b",
    r"\gcd(m, n), \operatorname{lcm}(m, n)",
    r"\mid, \nmid, \shortmid, \nshortmid"
);

round_trip!(
    radicals,
    r"\surd, \sqrt{2}, \sqrt[n]{2}, \sqrt[3]{\frac{x^3+y^3}{2}}"
);

round_trip!(
    operators,
    r"+, -, \pm, \mp, \dotplus",
    r"\times, \div, \divideontimes, /, \backslash",
    r"\cdot, * \ast, \star, \circ, \bullet",
    r"\boxplus, \boxminus, \boxtimes, \boxdot",
    r"\oplus, \ominus, \otimes, \oslash, \odot",
    r"\circleddash, \circledcirc, \circledast",
    r"\bigoplus, \bigotimes, \bigodot"
);

round_trip!(
    sets,
    r"\{ \}, \emptyset, \varnothing",
    r"\in, \notin \not\in, \ni, \not\ni",
    r"\cap, \Cap, \sqcap, \bigcap",
    r"\cup, \Cup, \sqcup, \bigcup, \bigsqcup, \uplus, \biguplus",
    r"\setminus, \smallsetminus, \times",
    r"\subset, \Subset, \sqsubset",
    r"\supset, \Supset, \sqsupset",
    r"\subseteq, \nsubseteq, \subsetneq, \varsubsetneq, \sqsubseteq",
    r"\supseteq, \nsupseteq, \supsetneq, \varsupsetneq, \sqsupseteq",
    r"\subseteqq, \nsubseteqq, \subsetneqq, \varsubsetneqq",
    r"\supseteqq, \nsupseteqq, \supsetneqq, \varsupsetneqq"
);

round_trip!(
    relations,
    r"=, \ne, \neq, \equiv, \not\equiv",
    r"\doteq, \doteqdot, \overset{\underset{\mathrm{def}}{}}{=}, :=",
    r"\sim, \nsim, \backsim, \thicksim, \simeq, \backsimeq, \eqsim, \cong, \ncong",
    r"\approx, \thickapprox, \approxeq, \asymp, \propto, \varpropto",
    r"<, \nless, \ll, \not\ll, \lll, \not\lll, \lessdot",
    r">, \ngtr, \gg, \not\gg, \ggg, \not\ggg, \gtrdot",
    r"\le, \leq, \lneq, \leqq, \nleq, \nleqq, \lneqq, \lvertneqq",
    r"\ge, \geq, \gneq, \geqq, \ngeq, \ngeqq, \gneqq, \gvertneqq",
    r"\lessgtr, \lesseqgtr, \lesseqqgtr, \gtrless, \gtreqless, \gtreqqless",
    r"\leqslant, \nleqslant, \eqslantless",
    r"\geqslant, \ngeqslant, \eqslantgtr",
    r"\lesssim, \lnsim, \lessapprox, \lnapprox",
    r"\gtrsim, \gnsim, \gtrapprox, \gnapprox",
    r"\prec, \nprec, \preceq, \npreceq, \precneqq",
    r"\succ, \nsucc, \succeq, \nsucceq, \succneqq",
    r"\preccurlyeq, \curlyeqprec",
    r"\succcurlyeq, \curlyeqsucc",
    r"\precsim, \precnsim, \precapprox, \precnapprox",
    r"\succsim, \succnsim, \succapprox, \succnapprox"
);

round_trip!(
    geometric,
    r"\parallel, \nparallel, \shortparallel, \nshortparallel",
    r"\perp, \angle, \sphericalangle, \measuredangle, 45^\circ",
    r"\Box, \square, \blacksquare, \diamond, \Diamond, \lozenge, \blacklozenge, \bigstar",
    r"\bigcirc, \triangle, \bigtriangleup, \bigtriangledown",
    r"\vartriangle, \triangledown",
    r"\blacktriangle, \blacktriangledown, \blacktriangleleft, \blacktriangleright"
);

round_trip!(
    logic,
    r"\forall, \exists, \nexists",
    r"\therefore, \because, \And",
    r"\lor, \vee, \curlyvee, \bigvee",
    r"\land, \wedge, \curlywedge, \bigwedge",
    r"\bar{q}, \bar{abc}, \overline{q}, \overline{abc}",
    r"\lnot, \neg, \not\operatorname{R}, \bot, \to",
    r"\vdash, \dashv, \vDash, \Vdash, \models",
    r"\Vvdash, \nvdash, \nVdash, \nvDash, \nVDash",
    r"\ulcorner, \urcorner, \llcorner, \lrcorner"
);

round_trip!(
    arrows,
    r"\Rrightarrow, \Lleftarrow",
    r"\Rightarrow, \nRightarrow, \Longrightarrow, \implies",
    r"\Leftarrow, \nLeftarrow, \Longleftarrow",
    r"\Leftrightarrow, \nLeftrightarrow, \Longleftrightarrow, \iff",
    r"\Uparrow, \Downarrow, \Updownarrow",
    r"\rightarrow, \to, \nrightarrow, \longrightarrow",
    r"\leftarrow, \gets, \nleftarrow, \longleftarrow",
    r"\leftrightarrow, \nleftrightarrow, \longleftrightarrow",
    r"\uparrow, \downarrow, \updownarrow",
    r"\nearrow, \swarrow, \nwarrow, \searrow",
    r"\mapsto, \longmapsto",
    r"\rightharpoonup \rightharpoondown \leftharpoonup \leftharpoondown \upharpoonleft \upharpoonright \downharpoonleft \downharpoonright \rightleftharpoons \leftrightharpoons",
    r"\curvearrowleft \circlearrowleft \Lsh \upuparrows \rightrightarrows \rightleftarrows \rightarrowtail \looparrowright",
    r"\curvearrowright \circlearrowright \Rsh \downdownarrows \leftleftarrows \leftrightarrows \leftarrowtail \looparrowleft",
    r"\hookrightarrow \hookleftarrow \multimap \leftrightsquigarrow \rightsquigarrow \twoheadrightarrow \twoheadleftarrow"
);

round_trip!(
    special,
    r"\amalg \P \S \% \dagger \ddagger \ldots \cdots \vdots \ddots",
    r"\smile \frown \wr \triangleleft \triangleright",
    r"\diamondsuit, \heartsuit, \clubsuit, \spadesuit, \Game, \flat, \natural, \sharp"
);

round_trip!(
    unsorted,
    r"\diagup \diagdown \centerdot \ltimes \rtimes \leftthreetimes \rightthreetimes",
    r"\eqcirc \circeq \triangleq \bumpeq \Bumpeq \doteqdot \risingdotseq \fallingdotseq",
    r"\intercal \barwedge \veebar \doublebarwedge \between \pitchfork",
    r"\vartriangleleft \ntriangleleft \vartriangleright \ntriangleright",
    r"\trianglelefteq \ntrianglelefteq \trianglerighteq \ntrianglerighteq"
);

round_trip!(
    should_panic,
    unsupported,
    r"\N \R \Z \C \Q",
    r"\AA",
    r"\O \empty"
);

round_trip!(
    greek_alphabet,
    r"\Alpha \Beta \Gamma \Delta \Epsilon \Zeta \Eta \Theta",
    r"\Iota \Kappa \Lambda \Mu \Nu \Xi \Omicron \Pi",
    r"\Rho \Sigma \Tau \Upsilon \Phi \Chi \Psi \Omega",
    r"\alpha \beta \gamma \delta \epsilon \zeta \eta \theta",
    r"\iota \kappa \lambda \mu \nu \xi \omicron \pi",
    r"\rho \sigma \tau \upsilon \phi \chi \psi \omega",
    r"\varGamma \varDelta \varTheta \varLambda \varXi \varPi \varSigma \varPhi \varUpsilon \varOmega",
    r"\varepsilon \digamma \varkappa \varpi \varrho \varsigma \vartheta \varphi"
);

round_trip!(hebrew_symbols, r"\aleph \beth \gimel \daleth");

round_trip!(
    blackboard_bold,
    r"\mathbb{ABCDEFGHI}",
    r"\mathbb{JKLMNOPQR}",
    r"\mathbb{STUVWXYZ}"
);

round_trip!(
    boldface,
    r"\mathbf{ABCDEFGHI}",
    r"\mathbf{JKLMNOPQR}",
    r"\mathbf{STUVWXYZ}",
    r"\mathbf{abcdefghijklm}",
    r"\mathbf{nopqrstuvwxyz}",
    r"\mathbf{0123456789}"
);

round_trip!(
    boldface_greek,
    r"\boldsymbol{\Alpha \Beta \Gamma \Delta \Epsilon \Zeta \Eta \Theta}",
    r"\boldsymbol{\Iota \Kappa \Lambda \Mu \Nu \Xi \Omicron \Pi}",
    r"\boldsymbol{\Rho \Sigma \Tau \Upsilon \Phi \Chi \Psi \Omega}",
    r"\boldsymbol{\alpha \beta \gamma \delta \epsilon \zeta \eta \theta}",
    r"\boldsymbol{\iota \kappa \lambda \mu \nu \xi \omicron \pi}",
    r"\boldsymbol{\rho \sigma \tau \upsilon \phi \chi \psi \omega}",
    r"\boldsymbol{\varepsilon\digamma\varkappa\varpi}",
    r"\boldsymbol{\varrho\varsigma\vartheta\varphi}"
);

round_trip!(italics, r"\mathit{0123456789}");

round_trip!(
    greek_italics,
    r"\mathit{\Alpha \Beta \Gamma \Delta \Epsilon \Zeta \Eta \Theta}",
    r"\mathit{\Iota \Kappa \Lambda \Mu \Nu \Xi \Omicron \Pi}",
    r"\mathit{\Rho \Sigma \Tau \Upsilon \Phi \Chi \Psi \Omega}"
);

round_trip!(
    greek_uppercase_boldface_italics,
    r"\boldsymbol{\varGamma \varDelta \varTheta \varLambda}",
    r"\boldsymbol{\varXi \varPi \varSigma \varUpsilon \varOmega}"
);

round_trip!(
    roman_typeface,
    r"\mathrm{ABCDEFGHI}",
    r"\mathrm{JKLMNOPQR}",
    r"\mathrm{STUVWXYZ}",
    r"\mathrm{abcdefghijklm}",
    r"\mathrm{nopqrstuvwxyz}",
    r"\mathrm{0123456789}"
);

round_trip!(
    sans_serif,
    r"\mathsf{ABCDEFGHI}",
    r"\mathsf{JKLMNOPQR}",
    r"\mathsf{STUVWXYZ}",
    r"\mathsf{abcdefghijklm}",
    r"\mathsf{nopqrstuvwxyz}",
    r"\mathsf{0123456789}"
);

round_trip!(
    sans_serif_greek,
    r"\mathsf{\Alpha \Beta \Gamma \Delta \Epsilon \Zeta \Eta \Theta}",
    r"\mathsf{\Iota \Kappa \Lambda \Mu \Nu \Xi \Omicron \Pi}",
    r"\mathsf{\Rho \Sigma \Tau \Upsilon \Phi \Chi \Psi \Omega}"
);

round_trip!(
    calligraphiy,
    r"\mathcal{ABCDEFGHI}",
    r"\mathcal{JKLMNOPQR}",
    r"\mathcal{STUVWXYZ}",
    r"\mathcal{abcdefghi}",
    r"\mathcal{jklmnopqr}",
    r"\mathcal{stuvwxyz}"
);

round_trip!(
    fraktur,
    r"\mathfrak{ABCDEFGHI}",
    r"\mathfrak{JKLMNOPQR}",
    r"\mathfrak{STUVWXYZ}",
    r"\mathfrak{abcdefghijklm}",
    r"\mathfrak{nopqrstuvwxyz}",
    r"\mathfrak{0123456789}"
);

// TODO: Support `\scriptstyle`
round_trip!(small_script, r"{\scriptstyle\text{abcdefghijklm}}");

round_trip!(parentheses, r"\left ( \frac{a}{b} \right )");

round_trip!(
    brackets,
    r"\left [ \frac{a}{b} \right ]",
    r"\left \lbrack \frac{a}{b} \right \rbrack"
);

round_trip!(
    braces,
    r"\left \{ \frac{a}{b} \right \}",
    r"\left \lbrace \frac{a}{b} \right \rbrace"
);

round_trip!(angle_brackets, r"\left \langle \frac{a}{b} \right \rangle");

round_trip!(
    bars_and_double_bars,
    r"\left | \frac{a}{b} \right \vert",
    r"\left \| \frac{a}{b} \right \Vert"
);

round_trip!(
    floor_and_ceiling,
    r"\left \lfloor \frac{a}{b} \right \rfloor",
    r"\left \lceil \frac{a}{b} \right \rceil"
);

round_trip!(
    slashes_and_backslashes,
    r"\left / \frac{a}{b} \right \backslash"
);

round_trip!(
    up_down_updown_arrows,
    r"\left \uparrow \frac{a}{b} \right \downarrow",
    r"\left \Uparrow \frac{a}{b} \right \Downarrow",
    r"\left \updownarrow \frac{a}{b} \right \Updownarrow"
);

round_trip!(
    mixed,
    r"\left [ 0,1 \right )",
    r"\left \langle \psi \right |"
);

round_trip!(no_delimiter, r"\left . \frac{A}{B} \right \} \to X");

round_trip!(
    delimiter_sizes,
    r"( \bigl( \Bigl( \biggl( \Biggl( \dots \Biggr] \biggr] \Bigr] \bigr] ]",
    r"\{ \bigl\{ \Bigl\{ \biggl\{ \Biggl\{ \dots \Biggr\rangle \biggr\rangle \Bigr\rangle \bigr\rangle \rangle",
    r"\| \big\| \Big\| \bigg\| \Bigg\| \dots \Bigg| \bigg| \Big| \big| |",
    r"\lfloor \bigl\lfloor \Bigl\lfloor \biggl\lfloor \Biggl\lfloor \dots \Biggr\rceil \biggr\rceil \Bigr\rceil \bigr\rceil \rceil",
    r"\uparrow \big\uparrow \Big\uparrow \bigg\uparrow \Bigg\uparrow \dots \Bigg\Downarrow \bigg\Downarrow \Big\Downarrow \big\Downarrow \Downarrow",
    r"\updownarrow \big\updownarrow \Big\updownarrow \bigg\updownarrow \Bigg\updownarrow \dots \Bigg\Updownarrow \bigg\Updownarrow \Big\Updownarrow \big\Updownarrow \Updownarrow",
    r"/ \big/ \Big/ \bigg/ \Bigg/ \dots \Bigg\backslash \bigg\backslash \Big\backslash \big\backslash \backslash"
);
