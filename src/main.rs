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


/// Check an ABC file, from STDIN to STDOUT.
fn main_check() {
    let chars = get_stdin().chars().collect::<Vec<char>>();
    let (num_errors, num_unshown, message) = abc_lexer::error_message(&chars);

    if num_errors > 0 {
        if num_errors == 1 {
            println!("There was {} error!", num_errors);
        } else {
            println!("There were {} errors!", num_errors);
        }

        println!("{}", message);

        // Don't expect this to happen but explain if it does.
        if num_unshown > 0 {
            println!("{} errors weren't shown", num_unshown);
        }
    } else {
        println!("All good!");
        println!("But here's the message anyway:");
        println!("{}", message);
    }

}

fn main_unrecognised() {
    println!(
        "Unrecognised command. Try:
 - check"
    );
}

fn main() {
    let mut args = env::args();

    match args.nth(1) {
        Some(first) => {
            match first.as_ref() {
                "check" => main_check(),
                _ => main_unrecognised(),
            }
        }
        _ => main_unrecognised(),
    }
}
