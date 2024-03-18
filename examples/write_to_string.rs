use pulldown_latex::parser::Parser;

static LATEX: &str = r#"
\forall \epsilon > 0, \exists \delta > 0,
\forall x \in \mathbb{R} \qquad |x - c| < \delta \implies |f(x) - L| < \epsilon.
"#;

fn main() {
    let parser = Parser::new(LATEX);
    let mut mathml = String::new();
    // TODO: Change That!!!
    let events = parser.collect::<Result<Vec<_>, _>>().inspect_err(|e| {
        eprintln!("Error while parsing: {}", e);
    }).unwrap();
    
    match pulldown_latex::mathml::push_html(&mut mathml, events.into_iter()) {
        Ok(()) => println!("{}", mathml),
        Err(e) => eprintln!("Error while rendering: {}", e),
    }
}
