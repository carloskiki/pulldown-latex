# Suffix Handling

This file documents the design decisions made surrounding suffix handling and the suffixes' location around
the nucleus of the math atom.

## Suffix Parsing

The suffix parser first checks for directives about suffix placement, i.e. `\limits` and `\nolimits`,
if the `allow_suffix_modifiers` flag is set on the parser state. If the flag is set, and if more than one directive is found,
the last one takes effect, as per the [`amsmath docs`][amsdocs] (section 7.3). If the flag is not set, and a limit modifying
directive is found, the parser emits an error.

## Suffix Rendering

The attribute `movablelimits = "false"` is set on operators that would be affected by this attributes' default effect.
The events `Script::Movable*` adapt to whether the renderer is in inline or display mode.

[amsdocs]: https://mirror.its.dal.ca/ctan/macros/latex/required/amsmath/amsldoc.pdf
