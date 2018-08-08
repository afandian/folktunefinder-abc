use std::io::{self, Read};
use std::env;
extern crate tiny_http;
extern crate regex;

mod abc_lexer;
mod archive;
mod cluster;
mod geometry;
mod midi;
mod ngram;
mod text;
// mod tune_ast;
mod tune_ast_three;
mod viz;
mod music;
mod typeset;
// mod typeset2;
mod svg;
mod storage;
mod server;
mod application;
mod relations;

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
fn main_check(_application: &application::Application) {
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
fn main_typeset(_application: &application::Application) {
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

    // println!("{}", svg);
}

/// Visualise an ABC file. Whatever that means.
fn main_viz(_application: &application::Application) {
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

fn main_scan(application: &mut application::Application) {
    eprintln!("Start scan...");
    application.ensure_load_tunes();
    match application.tune_store {
        Some(ref mut tune_store) => tune_store.scan(),
        _ => (),
    }
    eprintln!("Finished scan!");
}

fn main_server(application: &mut application::Application) {
    eprintln!("Start server");
    application.ensure_load_tunes();

    server::main(application);
}

fn main_unrecognised(_application: &application::Application) {
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

    let mut application = application::Application::new();

    match args.nth(1) {
        Some(first) => {
            match first.as_ref() {
                "db_scan" => main_scan(&mut application),
                "db_server" => main_server(&mut application),
                "check" => main_check(&application),
                "typeset" => main_typeset(&application),
                "viz" => main_viz(&application),
                _ => main_unrecognised(&application),
            }
        }
        _ => main_unrecognised(&application),
    }
}
