use pulldown_latex::{mathml::push_mathml, parser::{Parser, Storage}};

static INPUT_LATEX: &str = r#"
\forall \epsilon > 0, \exists \delta > 0, \text{s.t.}
\forall x \in \mathbb{R} \qquad |x - c| < \delta \implies |f(x) - L| < \epsilon.
"#;

fn main() {
    let storage = Storage::new();
    let parser = Parser::new(INPUT_LATEX, &storage);
    let mut mathml = String::new();
    let config = Default::default();

    match push_mathml(&mut mathml, parser, config) {
        Ok(()) => println!("{}", mathml),
        Err(e) => eprintln!("Error while rendering: {}", e),
    }
}
