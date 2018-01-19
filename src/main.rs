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
// mod tune_ast;
mod tune_ast_two;
mod viz;
mod music;
mod typeset;
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

    // let ast = tune_ast::read_from_lexer(abc_lexer::Lexer::new(&chars));
// 
    // let typeset = typeset::typeset_from_ast(ast);

    // println!("{}", typeset);
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



fn main_unrecognised() {
    println!(
        "Unrecognised command. Try:
 - check
 - typeset
 - viz"
    );
}

fn main() {
    let mut args = env::args();

    match args.nth(1) {
        Some(first) => {
            match first.as_ref() {
                "check" => main_check(),
                "typeset" => main_typeset(),
                "viz" => main_viz(),
                _ => main_unrecognised(),
            }
        }
        _ => main_unrecognised(),
    }
}
