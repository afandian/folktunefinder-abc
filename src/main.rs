use std::env;
use std::io::{self, Read};
use std::sync::Arc;
extern crate regex;
extern crate tiny_http;

mod abc_lexer;
mod music;
mod relations;
mod representations;
mod server;
mod storage;
mod svg;
mod tune_ast_three;
mod typeset;
mod pitch;


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
}

/// Check an ABC file, from STDIN to STDOUT.
fn main_typeset() {
    let stdin = get_stdin();
    let chars = stdin.chars().collect::<Vec<char>>();
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

    let ast = representations::abc_to_ast(&stdin);
    let svg = representations::ast_to_svg(&ast);

    println!("{}", svg);
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

// Analyze and group tunes, save groups to disk.
// Work in progress.
fn main_group() {
    eprintln!("Groups.");

    eprintln!("Load...");
    let tune_cache_path = storage::tune_cache_path().expect("Base directory config not supplied.");
    let abcs = storage::load(&tune_cache_path);

    eprintln!("Parse...");
    let abcs_arc = Arc::new(abcs);
    let asts = representations::abc_to_ast_s(abcs_arc);

    eprintln!("Pitches...");
    let pitches = representations::ast_to_pitches_s(&asts);

    eprintln!("Intervals...");
    let intervals = representations::pitches_to_intervals_s(&pitches);

    eprintln!("Interval histograms...");
    let interval_histograms = representations::intervals_to_interval_histogram_s(&intervals);

    // Now do something with these!

    eprintln!("Got {} results", interval_histograms.len());
}


fn main_unrecognised() {
    eprintln!(
        "Unrecognised command. Try:
 - db_scan
 - db_server
 - check
 - typeset"
    );
}

fn main() {
    let mut args = env::args();

    match args.nth(1) {
        Some(first) => match first.as_ref() {
            "db_scan" => main_scan(),
            "db_server" => main_server(),
            "db_group" => main_group(),
            "check" => main_check(),
            "typeset" => main_typeset(),
            _ => main_unrecognised(),
        },
        _ => main_unrecognised(),
    }
}
