//! Representations
//! Functions that convert from one representation of a tune to another.
//! Intended to be chained, cached, etc.

use abc_lexer;
use pitch;
use relations;
use std::collections::HashMap;
use std::sync::mpsc::channel;
use std::sync::Arc;
use std::thread;
use tune_ast_three;
use typeset;

// This gives the best performance for 200,000 tunes.
const THREADS: u32 = 8;

// Convert an ABC tune as a String into an Abstract Syntax Tree.
pub fn abc_to_ast(content: &String) -> tune_ast_three::Tune {
    let chars = content.chars().collect::<Vec<char>>();
    tune_ast_three::read_from_lexer(abc_lexer::Lexer::new(&chars))
}

// Convert a HashMap of ABC tunes as a String into Abstract Syntax Trees.
// Take an ARC wrapped value so it works with threads.
pub fn abc_to_ast_s(inputs_arc: Arc<HashMap<u32, String>>) -> HashMap<u32, tune_ast_three::Tune> {
    let mut result = HashMap::with_capacity(inputs_arc.clone().len());

    let (tx, rx) = channel();
    for thread_i in 0..THREADS {
        let tx_clone = tx.clone();
        let inputs_clone = inputs_arc.clone();

        thread::spawn(move || {
            let mut partition_result =
                HashMap::with_capacity(inputs_clone.len() / THREADS as usize);

            for (i, content) in inputs_clone.iter() {
                if (i % THREADS) == thread_i {
                    let ast = abc_to_ast(content);
                    partition_result.insert(*i, ast);
                }
            }

            tx_clone.send(partition_result);
        });
    }

    for _ in 0..THREADS {
        let partition_result = rx.recv().unwrap();
        eprintln!("Got chunk of {}", partition_result.len());
        result.extend(partition_result);
    }

    result
}

// Convert an Abstract Syntax Tree into an SVG.
pub fn ast_to_svg(ast: &tune_ast_three::Tune) -> String {
    let typeset_page = typeset::typeset_from_ast(ast);
    typeset::render_page(typeset_page)
}

pub fn ast_to_pitches(ast: &tune_ast_three::Tune) -> Vec<u8> {
    pitch::build_pitch_sequence(ast)
}

pub fn ast_to_pitches_s(inputs: &HashMap<u32, tune_ast_three::Tune>) -> HashMap<u32, Vec<u8>> {
    let mut result = HashMap::with_capacity(inputs.len());

    for (id, content) in inputs.iter() {
        result.insert(*id, ast_to_pitches(content));
    }

    result
}

pub fn pitches_to_intervals(inputs: &Vec<u8>) -> Vec<i16> {
    pitch::pitch_seq_to_intervals(inputs)
}

pub fn pitches_to_intervals_s(inputs: &HashMap<u32, Vec<u8>>) -> HashMap<u32, Vec<i16>> {
    let mut result = HashMap::with_capacity(inputs.len());

    for (id, content) in inputs.iter() {
        result.insert(*id, pitches_to_intervals(content));
    }

    result
}

pub fn intervals_to_interval_histogram(inputs: &Vec<i16>) -> [f32; pitch::HISTOGRAM_WIDTH] {
    pitch::build_interval_histogram(inputs)
}

pub fn intervals_to_interval_histogram_s(
    inputs: &HashMap<u32, Vec<i16>>,
) -> HashMap<u32, [f32; pitch::HISTOGRAM_WIDTH]> {
    let mut result = HashMap::with_capacity(inputs.len());

    for (id, content) in inputs.iter() {
        result.insert(*id, intervals_to_interval_histogram(content));
    }

    result
}

pub fn intervals_to_binary_vsm(
    inputs: &HashMap<u32, Vec<i16>>,
) -> relations::IntervalWindowBinaryVSM {
    let top_id = inputs.keys().max().unwrap();

    let mut result = relations::IntervalWindowBinaryVSM::new(16127, *top_id as usize);

    for (id, content) in inputs.iter() {
        result.add(*id as usize, content);
    }

    result
}