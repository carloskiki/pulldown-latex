use leptos::*;
use pulldown_latex::{config::DisplayMode, Storage, push_mathml, Parser, RenderConfig};

#[component]
pub fn App() -> impl IntoView {
    let input_ref: NodeRef<html::Textarea> = create_node_ref();
    let (output_math, set_output_math) = create_signal(String::new());
    
    let update_input = move |_| {
        let input_value = input_ref.get().unwrap().value();
        let storage = Storage::new();
        let parser = Parser::new(&input_value, &storage);
        let config = RenderConfig {
            display_mode: DisplayMode::Block,
            ..Default::default()
        };
        set_output_math.update(|out_math| {
            out_math.clear();
            push_mathml(out_math, parser, config).unwrap();
        });
    };
    
    view! {
        <h1>"pulldown-latex"</h1>
        <p>"A pull parser for LaTeX math rendering to MathML."</p>
        <div id="container">
            <textarea rows="20" cols="64" on:input=update_input node_ref=input_ref />
            <div inner_html=output_math id="math-output">
            </div>
        </div>
    }
}
