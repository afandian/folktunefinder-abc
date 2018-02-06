//! FolkTuneFinder ABC Tools Application centre of gravity.

use storage;
use std::str;
use tune_ast_three;
use typeset;
use abc_lexer;

pub struct Application {
    tune_store: storage::TuneStore,
}

impl Application {
    pub fn new() -> Application {
        let mut tune_store = storage::TuneStore::new();

        Application { tune_store }
    }

    /// Retrieve this tune's ABC.
    pub fn get_abc(&self, tune_id: u32) -> Option<String> {
        if let Some(result) = self.tune_store.tune_cache.get_tune(&tune_id) {
            // TODO Allocating a vec and a String when we don't need to.
            if let Ok(result) = String::from_utf8(result.to_vec()) {
                Some(result)
            } else {
                None
            }
        } else {
            None
        }
    }

    /// Render this tune's ABC into SVG.
    pub fn get_svg(&self, tune_id: u32) -> Option<String> {
        if let Some(abc_result) = self.tune_store.tune_cache.get_tune_string(&tune_id) {
            let chars = abc_result.chars().collect::<Vec<char>>();
            let ast = tune_ast_three::read_from_lexer(abc_lexer::Lexer::new(&chars));
            let typeset_page = typeset::typeset_from_ast(ast);
            Some(typeset::render_page(typeset_page))
        } else {
            None
        }
    }
}
