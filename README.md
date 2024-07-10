# pulldown-latex

A pull parser for $\LaTeX$ parsing and `mathml` rendering.

This project is inspired `KaTeX`, `Temml`, `MathJax`, etc. It is actively maintained, and is in a stage of development where
95% of what `KaTeX` and the likes support is properly working and minimally tested. This software
should be functional for most use cases. However, it is not recommended for large scale production
use as more robust testing is required.

## Goals

__Follow modern LaTeX principles:__
Ideally, this library should be mostly compatible with `latex2e` and `amsmath`. The term
_mostly_ is used here to refer to the mathematical commands exposed by these packages; typesetting prose
is out of scope for this crate. Another consequence of this goal is that some plain-TeX commands that
are deprecated (e.g., `\atop`, `\over`, etc.) are not supported by this crate.

__Closely resemble conventional LaTeX:__
It is a goal for this crate to make efforts in generating aesthetic equations. This means that
the `mathml` output may be tweaked to make it resemble what `pdflatex`, `KaTeX` or `MathJax` outputs.

## Development Notes

### To Test
- [ ] All the things temml and katex test
- [ ] Cargo Mutants
- [ ] Errors

### TODO's/Known Bugs
__For 0.2__:
- [x] Correctly handle array columns with multiple vertical bars.
- [x] `\hline` and `\hdashline`.
- [ ] Color api rework
- [ ] Documentation.
__For later__:
- [ ] `Dimension` values are unecessarily big
- [ ] square braket matrices do not have equal spacing on the left and the right.
- [ ] italic numbers are not italic because they do not exist in unicode.
- [ ] `\bar` make a tiny bar, should be bigger.
- [ ] `\sideset`

## Unsupported Plain-TeX & LaTeX behavior

- Changing `catcode`s of characters
- `\if`* macros
- `^^_` & `^^[0-9a-f][0-9a-f]` as a way of specifying characters
- __Redefining active characters__
    This library currently only supports default active characters, and hence does not allow for the 
    definition of active characters.
- Implicit characters as whitespace tokens
    As in the TeXbook p. 265, Knuth specifies that a `space token` stands for an _explicit_ or _implicit_
    space. This library does not currently support _implicit_ space tokens when a `space token` is required.
- Use of internal values and parameters, such as registers, and things like `\tolerance`
    (See TeXbook p. 267 for a complete definition)
- `\magnification` parameter & `true` sizes
- Case insensitive keywords matching. 
    According to TeXbook p. 265, keywords such as `pt`, `em`, `true`, etc. are matched case insensitively (e.g.,
    `pT` would match `pt`). This library does not support this behavior, as keywords must match exactly (i.e., 
    `em`, `true`, `pt`, etc.).
- `fil` units
    TeX allows the use of `fil`(ll...) units, this library does not.
- `\outer` specifier on definitions
- `\csname` & `\endcsname`
- `\begingroup` and `{`, and `\endgroup` and `}` behave the same way; that is to say, 
    `\begingroup` and `\endgroup` do not have the property of "keeping the same mode" (TeXbook p. 275).
- All vertical list manipulation commands.
    Things like `\vskip`, `\vfil`, `\moveleft` etc.
- `\hfil`, `\hfill`
- `\over`, `\atop`, and all deprecated "fraction like" control sequences.

### Unsupported KaTeX/Temml Options

- Macros preamble
- Wrap
- Left equation numbers
- `colorIsTextColor`
- `ThrowOnError`
- `maxSize`
- `trust`
- `\toggle` groups

## Miscellaneous References & Tools
Sources used during the development of this crate. Any reference in code comments refer to
these links specifically.

- [TeXBook](https://visualmatheditor.equatheque.net/doc/texbook.pdf)
- [latex2e unofficial Reference](https://tug.org/texinfohtml/latex2e.html)
- [amsmath docs](https://www.latex-project.org/help/documentation/amsldoc.pdf)
- [Comprehensive symbol list](https://mirror.its.dal.ca/ctan/info/symbols/comprehensive/symbols-letter.pdf)
- [Unicode-math symbol list](https://mirror.its.dal.ca/ctan/macros/unicodetex/latex/unicode-math/unimath-symbols.pdf)
- [Unicode-math package page](https://ctan.org/pkg/unicode-math)
- [Font tester](https://fred-wang.github.io/MathFonts/)
- [Math Variant Selection](https://milde.users.sourceforge.net/LUCR/Math/math-font-selection.xhtml#math-styles)
- [Official Latex Unicode Data](https://github.com/latex3/unicode-data)

