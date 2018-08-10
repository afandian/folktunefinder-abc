//! Experimental AST where each voice is simply a string of tokens.

use abc_lexer as l;
use music;

#[derive(Debug)]
pub struct Tune {
    /// All the entities that fall outside of the tune structure, i.e. occur in the tune header.
    pub prelude: Vec<l::T>,

    pub voices: Vec<Vec<l::T>>,
}

// TODO SHOULD BE ENTITY?
// Would allow for attachment of accidentals etc.

impl Tune {
    pub fn new() -> Tune {
        Tune {
            prelude: vec![],
            voices: vec![],
        }
    }
}

/// Read from a Lexer and build a new AST.
pub fn read_from_lexer(lexer: l::Lexer) -> Tune {
    // Every Entity has an index.
    let mut i = 0;

    let mut tune = Tune::new();

    let mut finished_prelude = false;
    let mut current_sequence = vec![];

    // The base note length. This can change during the tune.
    let mut note_length = music::FractionalDuration(1, 4);

    for token in lexer.collect_tokens() {
        match token {
            l::T::KeySignature(pitch_class, mode) => {
                current_sequence.push(l::T::KeySignature(pitch_class, mode));

                // K marks the end of the prelude.
                if !finished_prelude {
                    tune.prelude = current_sequence;
                    finished_prelude = true;
                    current_sequence = vec![];
                }
            }

            // The "L:" token doesn't produce an entity, it just updates the running status.
            l::T::DefaultNoteLength(new_note_length) => note_length = new_note_length,

            l::T::Note(note) => {
                current_sequence.push(l::T::Note(note.resolve_duration(note_length)))
            }

            token => current_sequence.push(token),
        }
    }

    tune.voices.push(current_sequence);

    tune
}

// Heuristics:
// 1 - Remove consecutive beam breaks.
// 2 - Remove unused beam breaks, e.g. first thing in a sequence.
