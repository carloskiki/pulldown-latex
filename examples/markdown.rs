use pulldown_cmark::{html, CowStr, Event, Options, Parser as MarkdownParser};
use pulldown_latex::{
    config::DisplayMode, push_mathml, Parser as LatexParser, RenderConfig, Storage,
};

/// Converts latex to inline or block MathML.
fn latex_to_mathml(latex: &str, display_mode: DisplayMode) -> String {
    let storage = Storage::new();
    let parser = LatexParser::new(latex, &storage);
    let config = RenderConfig {
        display_mode,
        ..Default::default()
    };
    let mut mathml = String::new();
    push_mathml(&mut mathml, parser, config).expect("writing to a string should not fail");
    mathml
}

/// Converts Markdown input containing LaTeX to HTML and MathML.
fn markdown_to_html(input: &str) -> String {
    let parser = MarkdownParser::new_ext(input, Options::all()).map(|event| match event {
        Event::InlineMath(latex) => {
            Event::InlineHtml(CowStr::from(latex_to_mathml(&latex, DisplayMode::Inline)))
        }
        Event::DisplayMath(latex) => {
            Event::Html(CowStr::from(latex_to_mathml(&latex, DisplayMode::Block)))
        }
        event => event,
    });
    let mut html_output = String::new();
    html::push_html(&mut html_output, parser);
    html_output
}

fn main() {
    let markdown = r#"
# Math in Markdown

Euler's identity is $e^{i\pi} + 1 = 0$.

The Gaussian integral is:

$$\int_{-\infty}^{\infty} e^{-x^2}\,dx = \sqrt{\pi}$$
"#;

    println!("{}", markdown_to_html(markdown));
}
