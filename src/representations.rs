//! Representations
//! Functions that convert from one representation of a tune to another.
//! Intended to be chained, cached, etc.

use abc_lexer;
use features;
use pitch;
use relations;
use std::collections::HashMap;
use std::sync::mpsc::channel;
use std::sync::Arc;
use std::thread;
use storage;
use tune_ast_three;
use typeset;

use abc_lexer as l;

// This gives the best performance for 200,000 tunes.
const THREADS: u32 = 8;

// Convert an ABC tune as a String into an Abstract Syntax Tree.
pub fn abc_to_ast(content: &String) -> tune_ast_three::Tune {
    let chars = content.chars().collect::<Vec<char>>();
    tune_ast_three::read_from_lexer(abc_lexer::Lexer::new(&chars))
}

// Convert an Abstract Syntax Tree into an SVG.
pub fn ast_to_svg(ast: &tune_ast_three::Tune) -> String {
    let typeset_page = typeset::typeset_from_ast(ast);
    typeset::render_page(typeset_page)
}

pub fn ast_to_features(ast: &tune_ast_three::Tune) -> Vec<(String, String)> {
    features::extract_all_features(ast)
}
