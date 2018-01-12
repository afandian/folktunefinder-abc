//! Tune Abstract Syntax Tree
//! Turns an ABC token stream into a more useful structure.
//! Beacuse musical structures suffer from "overlapping markup" the tree can't have a strictly 
//! hierarchical structure. Instead, those features are represented as "milestone" entities.
//! (see https://en.wikipedia.org/wiki/Overlapping_markup )
//! So the "AST" is more of a LA(0) grammar over a sequence.


use abc_lexer as l;
use music;

/// Vocabulary of object types.
/// These are similar but different to the various lexer tokens.
#[derive(Debug)]
enum HeaderField {
    Area(String),
    Book(String),
    Composer(String),
    Discography(String),
    Filename(String),
    Group(String),
    History(String),
    Information(String),
    Notes(String),
    Origin(String),
    Source(String),
    Title(String),
    Words(String),
    X(String),
    Transcription(String),
    Metre(u32, u32),
    KeySignature(music::PitchClass, music::Mode),
}



#[derive(Debug)]
enum Entity {
    Barline(music::Barline),
    Note(music::Note),

    // Milestone entities.
    OpenBeam,
    CloseBeam,
}

#[derive(Debug)]
pub struct TuneAst {
    headers: Vec<HeaderField>,
    errors: Vec<(usize, l::LexError)>,

    entities: Vec<Entity>,
}


impl TuneAst {
    pub fn new() -> TuneAst {
        TuneAst {
            headers: vec![],
            errors: vec![],
            entities: vec![],
        }
    }
}

/// Read from a Lexer and build a new AST.
pub fn read_from_lexer(lexer: l::Lexer) -> TuneAst {
    // The sequence of tune entities.
    // This will mostly be BarLines and BeamGroups.
    // All notes will live in a BeamGroup, possibly as singletons.
    let mut headers: Vec<HeaderField> = vec![];
    let mut errors: Vec<(usize, l::LexError)> = vec![];
    let mut entities: Vec<Entity> = vec![];

    // Running state.

    // What offset in the entities is the start of the current beam group?
    // There's always a beam group in play.
    let mut start_of_beam_group: Option<usize> = None;

    for token in lexer {
        match token {
            // On error extract the index from the context. That's the only bit we need.
            // Keeping the context confers the lifetime of the underlying ABC char slice on the AST.
            // Coupling the AST to its source isn't desirable. The index is all we need to store.
            // Using it with the input to print errors can exist in a parent context.
            l::LexResult::Error(_, offset, error) => errors.push((offset, error)),

            // If there's a token we don't care about the context.
            l::LexResult::T(_, token) => {
                match token {

                    l::T::Terminal => (),
                    // TODO depending on tune section this may mean start a new line of music.
                    l::T::Newline => (),
                    l::T::Area(value) => headers.push(HeaderField::Area(value)),
                    l::T::Book(value) => headers.push(HeaderField::Book(value)),
                    l::T::Composer(value) => headers.push(HeaderField::Composer(value)),
                    l::T::Discography(value) => headers.push(HeaderField::Discography(value)),
                    l::T::Filename(value) => headers.push(HeaderField::Filename(value)),
                    l::T::Group(value) => headers.push(HeaderField::Group(value)),
                    l::T::History(value) => headers.push(HeaderField::History(value)),
                    l::T::Information(value) => headers.push(HeaderField::Information(value)),
                    l::T::Notes(value) => headers.push(HeaderField::Notes(value)),
                    l::T::Origin(value) => headers.push(HeaderField::Origin(value)),
                    l::T::Source(value) => headers.push(HeaderField::Source(value)),
                    l::T::Title(value) => headers.push(HeaderField::Title(value)),
                    l::T::Words(value) => headers.push(HeaderField::Words(value)),
                    l::T::X(value) => headers.push(HeaderField::X(value)),
                    l::T::Transcription(value) => headers.push(HeaderField::Transcription(value)),
                    l::T::Metre(numerator, denomenator) => {
                        headers.push(HeaderField::Metre(numerator, denomenator))
                    }
                    l::T::KeySignature(pitch_class, mode) => {
                        headers.push(HeaderField::KeySignature(pitch_class, mode))
                    }

                    l::T::Barline(barline) => {

                        // If we're in a beam group, close it.
                        if let Some(start_i) = start_of_beam_group {

                            let beam_length = entities.len() - start_i;
                            if beam_length > 1 {
                                entities.push(Entity::CloseBeam);
                                entities.insert(start_i, Entity::OpenBeam);
                            }
                        }

                        start_of_beam_group = None;
                        entities.push(Entity::Barline(barline));
                    }
                    l::T::Note(note) => {

                        // If we're not in a beam group then start one, as it's the first note
                        // we've seen since the last one.
                        match start_of_beam_group {
                            None => start_of_beam_group = Some(entities.len()),
                            _ => (),
                        }

                        entities.push(Entity::Note(note));
                    }
                    l::T::BeamBreak => {
                        // If there's already an open beam group, wrap it up and close it.
                        if let Some(start_i) = start_of_beam_group {
                            let beam_length = entities.len() - start_i;
                            if beam_length > 1 {
                                entities.push(Entity::CloseBeam);
                                entities.insert(start_i, Entity::OpenBeam);
                            }
                        }

                        // Start a new one.
                        start_of_beam_group = Some(entities.len());
                    }
                }
            }

        }
    }

    // Finally close if there's still an open beam group.
if let Some(start_i) = start_of_beam_group {
                            let beam_length = entities.len() - start_i;
                            if beam_length > 1 {
                                entities.push(Entity::CloseBeam);
                                entities.insert(start_i, Entity::OpenBeam);
                            }
                        }

    return TuneAst {
        headers,
        errors,
        entities,
    };
}
