use leptos::*;
use pulldown_latex::{config::DisplayMode, push_mathml, Parser, RenderConfig};

#[component]
pub fn App() -> impl IntoView {
    let input_ref: NodeRef<html::Textarea> = create_node_ref();
    let (output_math, set_output_math) = create_signal(String::new());
    
    let update_input = move |_| {
        let input_value = input_ref.get().unwrap().value();
        let parser = Parser::new(&input_value);
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
        <div>
            <textarea rows="10" cols="50" on:input=update_input node_ref=input_ref />
            <div inner_html=output_math>
            </div>
        </div>
    }
}
