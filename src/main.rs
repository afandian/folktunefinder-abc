use std::env;
use std::io::{self, Read};
use std::sync::mpsc::channel;
use std::sync::Arc;
use std::thread;
use std::time::SystemTime;

extern crate regex;
extern crate tiny_http;

mod abc_lexer;
mod music;
mod pitch;
mod relations;
mod representations;
mod server;
mod storage;
mod svg;
mod tune_ast_three;
mod typeset;

/// Get STDIN as a string.
fn get_stdin() -> String {
    let mut buffer = String::new();

    match io::stdin().read_to_string(&mut buffer) {
        Err(_) => panic!("Can't read input!"),
        _ => (),
    }

    buffer
}

/// Check an ABC file, print the AST.
fn main_ast() {
    // let chars = get_stdin().chars().collect::<Vec<char>>();
    let input = get_stdin();
    let ast = representations::abc_to_ast(&input);
    eprintln!("{:#?}", ast);
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
    } else {
        eprintln!("Ok!");
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

    let max_tune_id = storage::max_id(&abcs);
    eprintln!("Max tune id: {}", max_tune_id);

    eprintln!("Parse...");
    let abcs_arc = Arc::new(abcs);
    let asts = representations::abc_to_ast_s(abcs_arc);

    eprintln!("Pitches...");
    let pitches = representations::ast_to_pitches_s(&asts);

    eprintln!("Intervals...");
    let intervals = representations::pitches_to_intervals_s(&pitches);

    // The search is mostly about zipping through large amounts of contiguous memory
    // and doing simple bit manipulation, so too many threads may cause cache-thrashing 
    // and make things worse.   
    const THREADS : u32 = 4;

    let start = SystemTime::now();
    let mut interval_term_vsm = representations::intervals_to_binary_vsm(&intervals);
    let mut groups = relations::Grouper::with_max_id(max_tune_id as usize);
    let vsm_arc = Arc::new(interval_term_vsm);
    let (tx, rx) = channel();
    for thread_i in 0..THREADS {
        let tx_clone = tx.clone();
        let interval_term_vsm = vsm_arc.clone();
        eprintln!("Start thread: {}", thread_i);
        thread::spawn(move || {
            let mut groups = relations::Grouper::with_max_id(max_tune_id as usize);
            let mut a_count = 0;
            for a in 0..max_tune_id {
                if (a % THREADS) == thread_i {
                    let results = interval_term_vsm
                        .vsm
                        .search_by_id(a as usize, 0.8, relations::ScoreNormalization::Max)
                        .results();

                    for (b, score) in results {
                        groups.add(a as usize, b as usize);
                    }

                    a_count += 1;

                    if a_count % 1000 == 0 {
                        eprintln!(
                            "Done {} tunes (projected total {}) in thread {}...",
                            a_count,
                            a_count * THREADS,
                            thread_i
                        );
                    }
                }
            }

            tx_clone.send(groups);
        });
    }

    for _ in 0..THREADS {
        let thread_group = rx.recv().unwrap();
        groups.extend(thread_group);
    }
    let end = SystemTime::now();

    eprintln!("Took {:?}", end.duration_since(start));
    
    // This output is suitable for the current (legacy?) Clojure search engine.
    groups.print_debug();
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
            "ast" => main_ast(),
            "typeset" => main_typeset(),
            _ => main_unrecognised(),
        },
        _ => main_unrecognised(),
    }
}
