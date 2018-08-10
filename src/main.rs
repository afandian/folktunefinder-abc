use std::env;
use std::io::{self, Read};
extern crate regex;
extern crate tiny_http;

mod abc_lexer;
mod archive;
mod cluster;
mod geometry;
mod midi;
mod ngram;
mod text;
// mod tune_ast;
mod music;
mod tune_ast_three;
mod typeset;
mod viz;
// mod typeset2;
mod relations;
mod server;
mod storage;
mod svg;

/// Get STDIN as a string.
fn get_stdin() -> String {
    let mut buffer = String::new();

    match io::stdin().read_to_string(&mut buffer) {
        Err(_) => panic!("Can't read input!"),
        _ => (),
    }

    buffer
}

/// Check an ABC file, from STDIN to STDOUT.
fn main_check() {
    let chars = get_stdin().chars().collect::<Vec<char>>();
    let (num_errors, num_unshown, message) = abc_lexer::format_error_message_from_abc(&chars);

    if num_errors > 0 {
        if num_errors == 1 {
            eprintln!("There was {} error!", num_errors);
        } else {
            eprintln!("There were {} errors!", num_errors);
        }

        eprintln!("{}", message);

        // Don't expect this to happen but explain if it does.
        if num_unshown > 0 {
            eprintln!("{} errors weren't shown", num_unshown);
        }
        return;
    }

    // let ast = tune_ast_two::read_from_lexer(abc_lexer::Lexer::new(&chars));
    // println!("Tune: {:#?}", ast);
}

/// Check an ABC file, from STDIN to STDOUT.
fn main_typeset() {
    let chars = get_stdin().chars().collect::<Vec<char>>();
    let (num_errors, num_unshown, message) = abc_lexer::format_error_message_from_abc(&chars);

    if num_errors > 0 {
        if num_errors == 1 {
            eprintln!("There was {} error!", num_errors);
        } else {
            eprintln!("There were {} errors!", num_errors);
        }

        eprintln!("{}", message);

        // Don't expect this to happen but explain if it does.
        if num_unshown > 0 {
            eprintln!("{} errors weren't shown", num_unshown);
        }
        return;
    }

    let ast = tune_ast_three::read_from_lexer(abc_lexer::Lexer::new(&chars));

    let typeset_page = typeset::typeset_from_ast(ast);

    let svg = typeset::render_page(typeset_page);

    println!("{}", svg);
}

/// Visualise an ABC file. Whatever that means.
fn main_viz() {
    let chars = get_stdin().chars().collect::<Vec<char>>();
    let (num_errors, num_unshown, message) = abc_lexer::format_error_message_from_abc(&chars);

    if num_errors > 0 {
        if num_errors == 1 {
            eprintln!("There was {} error!", num_errors);
        } else {
            eprintln!("There were {} errors!", num_errors);
        }

        eprintln!("{}", message);

        // Don't expect this to happen but explain if it does.
        if num_unshown > 0 {
            eprintln!("{} errors weren't shown", num_unshown);
        }
        return;
    }

    // let ast = tune_ast_two::read_from_lexer(abc_lexer::Lexer::new(&chars));

    // let viz = viz::viz_from_ast(ast);

    // println!("{}", viz);
}

fn main_scan() {
    eprintln!("Start scan...");

    let tune_cache_path = storage::tune_cache_path().expect("Base directory config not supplied.");
    let base_path = env::var("BASE").expect("Base directory config not supplied.");

    let mut tune_cache = storage::load(&tune_cache_path);
    storage::scan(&mut tune_cache, &base_path);

    storage::save(&tune_cache, &tune_cache_path);
}

fn main_server() {
    let tune_cache_path = storage::tune_cache_path().expect("Base directory config not supplied.");
    let tune_cache = storage::load(&tune_cache_path);

    eprintln!("Start server");

    server::main(&tune_cache);
}

fn main_unrecognised() {
    eprintln!(
        "Unrecognised command. Try:
 - db_scan
 - db_server
 - check
 - typeset
 - viz"
    );
}

fn main() {
    let mut args = env::args();

    match args.nth(1) {
        Some(first) => match first.as_ref() {
            "db_scan" => main_scan(),
            "db_server" => main_server(),
            "check" => main_check(),
            "typeset" => main_typeset(),
            "viz" => main_viz(),
            _ => main_unrecognised(),
        },
        _ => main_unrecognised(),
    }
}
