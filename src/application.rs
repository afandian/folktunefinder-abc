//! FolkTuneFinder ABC Tools Application centre of gravity.

use storage;
use std::str;
use tune_ast_three;
use typeset;
use abc_lexer;
use std::collections::HashMap;


pub struct Application {
    /// Lazily loaded TuneStore
    pub tune_store: Option<storage::TuneStore>,

    /// Map to tune id to AST
    pub tune_asts: Option<HashMap<u32, Option<tune_ast_three::Tune>>>,
}

impl Application {
    pub fn new() -> Application {
        Application {
            tune_store: None,
            tune_asts: None,
        }
    }

    /// Loading the tunes isn't always required.
    /// Load on demand.
    pub fn ensure_load_tunes(&mut self) {
        if self.tune_store.is_none() {
            self.tune_store = Some(storage::TuneStore::new());
        }
    }

    pub fn ensure_ast_store(&mut self, diagnostic_log: bool) {
        self.ensure_load_tunes();

        let mut num_tunes = 0;
        let mut num_tunes_with_errors = 0;
        let mut num_errors = 0;

        if self.tune_asts.is_none() {
            if let Some(ref mut tune_store) = self.tune_store {
                for (tune_id, _) in tune_store.tune_cache.index.iter() {
                    // eprintln!("Parsing tune: {}", tune_id);

                    if let Some(ref abc) = tune_store.tune_cache.get_tune_string(&tune_id) {

                        let chars = abc.chars().collect::<Vec<char>>();
                        let ast = tune_ast_three::read_from_lexer(abc_lexer::Lexer::new(&chars));
                        let errors = abc_lexer::Lexer::new(&chars).collect_errors();

                        if errors.len() > 0 {
                            num_tunes_with_errors += 1;
                        }

                        num_errors += errors.len();
                        num_tunes += 1;

                        if diagnostic_log {
                            if num_tunes % 1000 == 0 {
                                eprintln!("Loaded {} tunes...", num_tunes);
                            }
                        }
                    } else {
                        eprintln!("Couldn't retrieve ABC for tune: {}", &tune_id)

                    }
                }
            }
        }

        if diagnostic_log {
            eprintln!(
                "Loaded {} tunes, of which {} had errors. Average {} errors per tune.",
                num_tunes,
                num_tunes_with_errors,
                num_errors / num_tunes
            );
        }
    }

    /// Retrieve this tune's ABC.
    /// Only retrieves something if it's been loaded.
    pub fn get_abc(&self, tune_id: u32) -> Option<String> {
        if let Some(ref tune_store) = self.tune_store {
            if let Some(ref result) = tune_store.tune_cache.get_tune(&tune_id) {
                // TODO Allocating a vec and a String when we don't need to.
                if let Ok(result) = String::from_utf8(result.to_vec()) {
                    Some(result)
                } else {
                    None
                }
            } else {
                None
            }
        } else {
            None
        }
    }

    /// Render this tune's ABC into SVG.
    /// Only retrieves something if it's been loaded.
    pub fn get_svg(&self, tune_id: u32) -> Option<String> {
        if let Some(ref tune_store) = self.tune_store {
            if let Some(abc_result) = tune_store.tune_cache.get_tune_string(&tune_id) {
                let chars = abc_result.chars().collect::<Vec<char>>();
                let ast = tune_ast_three::read_from_lexer(abc_lexer::Lexer::new(&chars));
                let typeset_page = typeset::typeset_from_ast(ast);
                Some(typeset::render_page(typeset_page))
            } else {
                None
            }
        } else {
            None
        }
    }
}
