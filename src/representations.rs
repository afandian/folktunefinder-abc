//! Representations
//! Functions that convert from one representation of a tune to another.
//! Intended to be chained, cached, etc.

use abc_lexer;
use tune_ast_three;
use typeset;
use pitch;

use std::collections::HashMap;

// Convert an ABC tune as a String into an Abstract Syntax Tree.
pub fn abc_to_ast(content: &String) -> tune_ast_three::Tune {
    let chars = content.chars().collect::<Vec<char>>();
    tune_ast_three::read_from_lexer(abc_lexer::Lexer::new(&chars))
}

pub fn abc_to_ast_s(inputs: &HashMap<u32, String>) -> HashMap<u32, tune_ast_three::Tune> {
	let mut result = HashMap::with_capacity(inputs.len());

	for (id, content) in inputs.iter() {
		result.insert(*id, abc_to_ast(content));
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

pub fn intervals_to_interval_histogram_s(inputs: &HashMap<u32, Vec<i16>>) -> HashMap<u32, [f32; pitch::HISTOGRAM_WIDTH]> {
	let mut result = HashMap::with_capacity(inputs.len());

	for (id, content) in inputs.iter() {
		result.insert(*id, intervals_to_interval_histogram(content));
	}

	result
}

