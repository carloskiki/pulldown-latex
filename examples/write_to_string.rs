use pulldown_latex::{mathml::push_html, parser::Parser};

static INPUT_LATEX: &str = r#"
\forall \epsilon > 0, \exists \delta > 0,
\forall x \in \mathbb{R} \qquad |x - c| < \delta \implies |f(x) - L| < \epsilon.
"#;

fn main() {
    let parser = Parser::new(INPUT_LATEX);
    let mut mathml = String::new();
    let config = Default::default();
    
    match push_html(&mut mathml, parser, config) {
        Ok(()) => println!("{}", mathml),
        Err(e) => eprintln!("Error while rendering: {}", e),
    }
}