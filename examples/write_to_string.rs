use pulldown_latex::parser::{Parser, InnerResult};

static LATEX: &str = r#"
\forall \epsilon > 0 \exists \delta > 0
\forall x \in \mathbb{R} \left( |x - c| < \delta \implies |f(x) - L| < \epsilon \right)
"#;

fn main() {
    let parser = Parser::new(LATEX);
    let mut mathml = String::new();
    // TODO: Change That!!!
    let events = parser.collect::<InnerResult<Vec<_>>>().inspect_err(|e| {
        eprintln!("Error: {}", e);
    }).unwrap();
    match pulldown_latex::mathml::push_html(&mut mathml, events.into_iter()) {
        Ok(()) => println!("{}", mathml),
        Err(e) => eprintln!("Error: {}", e),
    }
}
