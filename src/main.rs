use std::env;
use std::io::{self, Read};
use std::path::PathBuf;
use std::sync::mpsc::channel;
use std::sync::Arc;
use std::thread;
use std::time::SystemTime;

extern crate serde;
extern crate serde_json;

#[macro_use]
extern crate serde_derive;

extern crate handlebars;
extern crate regex;
extern crate tiny_http;
extern crate unidecode;
extern crate url;

mod abc_lexer;
mod end_to_end_test;
mod features;
mod music;
mod pitch;
mod relations;
mod representations;
mod search;
mod server;
mod storage;
mod svg;
mod text;
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

// Construct a path for the Clusters file from config.
pub fn clusters_path() -> Option<PathBuf> {
    let key = "BASE";
    match env::var(key) {
        Ok(base) => {
            let mut path = PathBuf::new();
            path.push(&base);
            path.push("clusters");

            Some(path)
        }
        _ => None,
    }
}

// Construct a path for the Tune Cache from config.
fn get_tune_cache_path() -> Option<PathBuf> {
    let key = "BASE";
    match env::var(key) {
        Ok(base) => {
            let mut path = PathBuf::new();
            path.push(&base);
            path.push("tunecache");

            Some(path)
        }
        _ => None,
    }
}

/// Check an ABC file, print the AST.
fn main_ast() {
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

// Scan ABCs into tunecache.
fn main_scan() {
    let tune_cache_path = get_tune_cache_path().expect("Base directory config not supplied.");
    let base_path = env::var("BASE").expect("Base directory config not supplied.");

    eprintln!("Refreshing tunecache...");
    let mut abcs = storage::ReadWriteCache::new(tune_cache_path).unwrap();
    eprintln!("Loading cache...");
    abcs.load_cache();
    eprintln!("Scanning ABC files...");
    abcs.scan_dir(&base_path);
    eprintln!("Saving tunecache...");
    abcs.flush();
    eprintln!("Done!");
}

// Validate the tunecache file's integrity.
fn main_validate() {
    let tune_cache_path = get_tune_cache_path().expect("Base directory config not supplied.");

    eprintln!("Load read-only...");
    let mut read_only_abcs = storage::ReadOnlyCache::new(tune_cache_path.clone()).unwrap();
    read_only_abcs.load_cache();

    eprintln!("Load read-write...");
    let mut read_write_abcs = storage::ReadWriteCache::new(tune_cache_path.clone()).unwrap();
    read_write_abcs.load_cache();

    eprintln!("Load Scanner");
    let mut scanner = storage::CacheScanner::new(tune_cache_path.clone());

    eprintln!("Compare...");
    let max_id = read_write_abcs.max_id();
    let mut errs = 0;
    for entry in scanner.iter() {
        let rw_str_value = read_write_abcs.get(entry.tune_id);
        let ro_str_value = read_only_abcs.get(entry.tune_id);

        // Do all 3 agree?
        let rw_ro_ok = rw_str_value == ro_str_value;
        let scanner_ok = Some(entry.content.clone()) == rw_str_value;

        if !rw_ro_ok || !scanner_ok {
            eprintln!("Tune: {}", entry.tune_id);
            eprintln!("RW val: {:?}", rw_str_value);
            eprintln!("RO val: {:?}", ro_str_value);
            eprintln!("Scanner val: {:?}", &entry.content);
            errs += 1;
        }
    }

    eprintln!("{} errors", errs);
}

fn main_server() {
    eprintln!("Server loading ABCs...");
    let tune_cache_path = get_tune_cache_path().expect("Base directory config not supplied.");

    // Load clusters outside the SearchEngine engine object as we might want to swap in different ones.
    eprintln!("Server loading clusters...");
    let groups = if let Some(path) = clusters_path() {
        relations::Clusters::load(&path)
    } else {
        eprintln!("Error! Couldn't work out where to find clusters file!");
        relations::Clusters::new()
    };

    eprintln!("Start server");

    let searcher = search::SearchEngine::new(
        tune_cache_path.clone(),
        groups,
        search::SearchEngineFeatures {
            index_text: true,
            index_melody_interval_term: true,
            index_features: true,
        },
    );
    server::main(searcher);
}

// Analyze and cluster tunes into groups, save cluster info to disk.
// Work in progress.
// TODO maybe use the SearchEngine object now?
fn main_cluster_preprocess() {
    eprintln!("Pre-process clusters.");

    let tune_cache_path = get_tune_cache_path().expect("Base directory config not supplied.");

    // Initialize a search engine with no clustering info.
    let clusters = relations::Clusters::new();
    let searcher = search::SearchEngine::new(
        tune_cache_path.clone(),
        clusters,
        search::SearchEngineFeatures {
            index_text: false,
            index_melody_interval_term: true,
            index_features: false,
        },
    );

    let max_tune_id = searcher.get_max_tune_id();

    // The search is mostly about zipping through large amounts of contiguous memory
    // and doing simple bit manipulation, so too many threads may cause cache-thrashing
    // and make things worse.
    const THREADS: u32 = 4;

    let start = SystemTime::now();

    let mut groups = relations::Clusters::with_max_id(max_tune_id as usize);

    let mut searcher_arc = Arc::new(searcher);
    let (tx, rx) = channel();
    for thread_i in 0..THREADS {
        let tx_clone = tx.clone();
        let searcher_clone = searcher_arc.clone();
        eprintln!("Start thread: {}", thread_i);
        thread::spawn(move || {
            let mut groups = relations::Clusters::with_max_id(max_tune_id as usize);
            let mut a_count = 0;
            for a in 0..max_tune_id {
                if (a % THREADS) == thread_i {
                    let results = &searcher_clone
                        .interval_term_vsm
                        .vsm
                        .search_by_id(a as usize, 0.8, relations::ScoreNormalization::Max)
                        .results();

                    for (b, _score) in results {
                        groups.add(a as usize, *b as usize);
                    }

                    a_count += 1;

                    if a_count % 100 == 0 {
                        eprintln!(
                            "Done {} tunes (projected total {}) in thread {}...",
                            a_count,
                            a_count * THREADS,
                            thread_i
                        );
                    }
                }
            }

            tx_clone.send(groups).unwrap();
        });
    }

    for _ in 0..THREADS {
        let thread_group = rx.recv().unwrap();
        groups.extend(thread_group);
    }
    let end = SystemTime::now();

    eprintln!("Took {:?}", end.duration_since(start));

    // This output is suitable for the current (legacy?) Clojure search engine.
    if let Some(path) = clusters_path() {
        groups.save(&path);
    } else {
        eprintln!("Error! Couldn't work out where to put the clusters file!");
    }

    groups.print_debug();
}

fn main_unrecognised() {
    eprintln!(
        "Unrecognised command. Try:
 - scan - Scan tune DB individual tunes into a single $BASE/tunecache file
 - validate - Validate integrity of the tunecache file.
 - cluster - Using the tunecache, cluster tunes and sage to $BASE/clusters file.
 - server - Run the server. run 'scan' and 'cluster' first!
 - check - Parse an ABC file from STDIN and check to see if it parses and get error messages.
 - ast - Parse an ABC file from  STDIN and pring out the abstract syntax tree.
 - typeset - Parse and ABC file from STDIN and print out an SVG file."
    );
}

fn main() {
    let mut args = env::args();

    // And now we can use it!
    let tune_cache_path = get_tune_cache_path().expect("Base directory config not supplied.");

    match args.nth(1) {
        Some(first) => match first.as_ref() {
            "scan" => main_scan(),
            "validate" => main_validate(),
            "server" => main_server(),
            "cluster" => main_cluster_preprocess(),
            "check" => main_check(),
            "ast" => main_ast(),
            "typeset" => main_typeset(),
            _ => main_unrecognised(),
        },
        _ => main_unrecognised(),
    }
}
