//! Representations
//! Functions that convert from one representation of a tune to another.
//! Intended to be chained, cached, etc.

use abc_lexer;
use features;
use pitch;
use relations;
use std::collections::HashMap;
use std::sync::mpsc::channel;
use std::sync::Arc;
use std::thread;
use storage;
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
pub fn abc_to_ast_s(inputs_arc: &Arc<storage::ABCCache>) -> HashMap<u32, tune_ast_three::Tune> {
    let mut result = HashMap::with_capacity(inputs_arc.clone().max_id() as usize);

    let (tx, rx) = channel();
    for thread_i in 0..THREADS {
        let tx_clone = tx.clone();
        let mut inputs_clone = inputs_arc.clone();

        thread::spawn(move || {
            let mut partition_result =
                HashMap::with_capacity(inputs_clone.max_id() as usize / THREADS as usize);

            // Need to take a mutable copy because `get` requires mut.
            // Bit messy, TODO tidy up.
            let mut inputs_clone: storage::ABCCache = (*inputs_clone).clone();
            for i in 0..inputs_clone.max_id() {
                if (i % THREADS) == thread_i {
                    if let Some(content) = inputs_clone.get(i) {
                        let ast = abc_to_ast(&content);
                        partition_result.insert(i, ast);
                    }
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

pub fn ast_to_features(ast: &tune_ast_three::Tune) -> Vec<(String, String)> {
    features::extract_all_features(ast)
}

// We think there will be about this many features.
// The number of features is small and in theory bounded.
// We want matchines to be exact with no collisions.
const FEATURES_SIZE: usize = 512;

pub fn asts_to_features_s(
    inputs: &HashMap<u32, tune_ast_three::Tune>,
) -> relations::FeaturesBinaryVSM {
    let top_id = inputs.keys().max().unwrap();
    let mut vsm = relations::FeaturesBinaryVSM::new(FEATURES_SIZE, *top_id as usize);

    for (id, content) in inputs.iter() {
        let features = ast_to_features(content);
        for (feature_type, feature_value) in features {
            vsm.add(*id as usize, feature_type, feature_value);
        }
    }

    vsm
}
