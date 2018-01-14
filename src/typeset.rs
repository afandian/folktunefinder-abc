use svg;
use tune_ast;

pub struct Typesetting {}

impl Typesetting {
    pub fn new() -> Typesetting {
        Typesetting {}
    }
}


pub fn typeset_from_ast(ast: tune_ast::TuneAst) -> String {
    let mut svg = svg::Drawing::new();

    // TODO STUB

    svg.render()
}
