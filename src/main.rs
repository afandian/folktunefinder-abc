use std::env;
use std::io::{self, Read};
use std::sync::Arc;
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

    let max_tune_id = storage::max_id(&abcs);
    eprintln!("Max tune id: {}", max_tune_id);

    eprintln!("Parse...");
    let abcs_arc = Arc::new(abcs);
    let asts = representations::abc_to_ast_s(abcs_arc);

    eprintln!("Pitches...");
    let pitches = representations::ast_to_pitches_s(&asts);

    eprintln!("Intervals...");
    let intervals = representations::pitches_to_intervals_s(&pitches);

    eprintln!("Interval histograms...");
    let interval_histograms = representations::intervals_to_interval_histogram_s(&intervals);

    // Now create preliminary groups based on pitch interval histogram euclidean distance.
    // Each of these groups is considered to be a superset of one or more subgroups.

    // Three methods, need to benchmark.

    // 1: All combinations.

    // let mut groups = relations::Grouper::new();
    // let mut a_count = 0;
    // for (id_a, histogram_a) in interval_histograms.iter() {
    //     eprintln!("Compare {}, done {}", id_a, a_count);
    //     a_count+= 1;

    //     for (id_b, histogram_b) in interval_histograms.iter() {
    //         if let None = groups.get(*id_b as usize) {
    //             let sim = pitch::sim_interval_histogram(histogram_a, histogram_b);

    //             if sim < 0.05 && sim > 0.0  {
    //                 groups.add(*id_a as usize, *id_b as usize);
    //             }
    //         }
    //     }
    // }
    // groups.print_debug();

    // 2: Same, but don't cover already-done ones.
    // May be quicker or slower than 1 depending on access patterns / internals of the hashmap interator.

    // let mut groups = relations::Grouper::with_max_id(max_tune_id as usize);
    // let mut a_count = 0;
    // for (id_a, histogram_a) in interval_histograms.iter() {
    //     eprintln!("Compare {}, done {}", id_a, a_count);
    //     a_count += 1;

    //     for id_b in (id_a + 1)..max_tune_id {
    //         if let Some(histogram_b) = interval_histograms.get(&id_b) {
    //             if let None = groups.get(id_b as usize) {
    //                 let sim = pitch::sim_interval_histogram(histogram_a, histogram_b);

    //                 if sim < 0.05 && sim > 0.0 {
    //                     groups.add(*id_a as usize, id_b as usize);
    //                 }
    //             }
    //         }
    //     }
    // }
    // groups.print_debug();

    // 3: Use the Grouper object to work out which which pairs of tunes to compare.
    // May be more efficient due to fewer comparisons. But may also lose out on random-access memory locality.

    // eprintln!("Grouping up to tune ID {}", max_tune_id);
    // let mut groups = relations::Grouper::with_max_id(max_tune_id as usize);
    // let mut prev_a_id = 0;
    // let mut a_count = 0;
    // while let Some(a_id) = groups.next_ungrouped_after(prev_a_id as u32) {
    //     prev_a_id = a_id;

    //     a_count += 1;

    //     // We may not have anything for this tune id.
    //     if let Some(a_hist) = interval_histograms.get(&(a_id as u32)) {
    //         let mut comparisons = 0;
    //         let mut group_members = 0;

    //         let mut prev_b_id = a_id;
    //         while let Some(b_id) = groups.next_ungrouped_after(prev_b_id as u32) {
    //             prev_b_id = b_id;
    //             comparisons += 1;

    //             if let Some(b_hist) = interval_histograms.get(&(b_id as u32)) {
    //                 let sim = pitch::sim_interval_histogram(a_hist, b_hist);

    //                 if sim < 0.05 && sim > 0.0 {
    //                     groups.add(a_id, b_id);
    //                     group_members += 1;
    //                 }
    //             }
    //         }

    //         eprintln!(
    //             "Compared id {} in {} comparisons, with {} other members, done {}",
    //             a_id, comparisons, group_members, a_count
    //         );
    //     }
    // }
    // groups.print_debug();

    // TODO: Load group before, only generate if missing, save after.

    // TODO further refinements of grouping.

    // eprintln!("Interval Term VSM...");
    let mut interval_term_vsm = representations::intervals_to_binary_vsm(&intervals);

    let mut groups = relations::Grouper::with_max_id(max_tune_id as usize);

    let mut a_count = 0;
    for id_a in 0..max_tune_id {
        // eprintln!("Compare tune: {}", id_a);
        let results = interval_term_vsm
            .vsm
            .search_by_id(id_a as usize, 0.8, relations::ScoreNormalization::Max)
            .results();
        if results.len() > 0 {
            eprintln!("Tune: {} => {:?}", id_a, results);
        }
        for (id_b, score) in results {
            // eprintln!("  {} = {}", id_b, score);
            groups.add(id_a as usize, id_b as usize);
        }
        a_count += 1;
        if a_count % 1000 == 0 {
            eprintln!("Done {} tunes...", a_count);
        }
    }
    groups.print_debug();

    // let mut groups = relations::Grouper::with_max_id(max_tune_id as usize);
    // for a in interval_histograms.keys() {
    //     eprintln!("Do {}", a);
    //     for b in interval_histograms.keys() {
    //         if a == b || groups.get(*b as usize).is_some() {
    //             continue;
    //         }
    //         let sim = interval_term_vsm.vsm.sim(*a, *b);
    //         if sim > 0.9 {
    //             eprintln!("-> {} = {}", b, sim);
    //             groups.add(*a as usize, *b as usize);
    //         }
    //     }
    // }

    // let mut groups = relations::Grouper::with_max_id(max_tune_id as usize);
    // let mut a_count = 0;
    //  for id_a in 0..max_tune_id     {
    //     eprint!("Compare {}, done {}", id_a, a_count);
    //     a_count += 1;

    //     let mut comprison_count = 0;
    //     for id_b in (id_a + 1)..max_tune_id {

    //         if groups.get(id_b as usize).is_some() {
    //             continue;
    //         }

    //         let sim = interval_term_vsm.vsm.sim(id_a, id_b);
    //         if sim > 0.8 {
    //             eprintln!("-> {} = {}", id_b, sim);
    //             groups.add(id_a as usize, id_b as usize);
    //         }
    //         comprison_count += 1;
    //         }

    //         eprintln!(" comparisons {}", comprison_count);
    //     }
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
