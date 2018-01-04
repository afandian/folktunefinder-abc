use std::io::{self, Read};
use std::env;

mod abc_lexer;
mod application;
mod archive;
mod cluster;
mod geometry;
mod midi;
mod ngram;
mod text;
mod tune_ast;
mod viz;

/// Get STDIN as a string.
fn get_stdin() -> String {
    let mut buffer = String::new();

    match io::stdin().read_to_string(&mut buffer) {
        Err(_) => panic!("Can't read input!"),
        _ => (),
    }

    buffer
}


/// Cleanup an ABC file, from STDIN to STDOUT.
fn main_cleanup() {
    let chars = get_stdin().chars().collect::<Vec<char>>();
    let lexer = abc_lexer::Lexer::new(&chars);
    let mut ast = tune_ast::TuneAst::new();

    tune_ast::read_from_lexer(lexer, &mut ast);

    if ast.num_errors() > 0 {
        println!("There were {} errors!", ast.num_errors());

        tune_ast::print_errors(&ast, &chars);
    }

}

fn main_unrecognised() {
    println!(
        "Unrecognised command. Try:
 - cleanup"
    );
}

fn main() {
    let mut args = env::args();

    match args.nth(1) {
        Some(first) => {
            match first.as_ref() {
                "cleanup" => main_cleanup(),
                _ => main_unrecognised(),
            }
        }
        _ => main_unrecognised(),
    }
}
