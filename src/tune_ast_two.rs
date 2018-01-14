//! Experimental AST that represents a tune as a hierachial tree structure. This may not be able to
//! represent the full syntax of ABC.
//! AST level:
//!  - Sequential entities as a sequence, e.g. notes, chords etc
//!  - Entities have decorations, e.g. accents
//!  - Non-sequential entities e.g. ties
//!  - Two separate data structures. Sequential entities have e.g. start, end references to entities


use abc_lexer as l;
use music;

#[derive(Debug)]
pub struct Tune<'a> {
    /// All the entities that fall outside of the tune structure, i.e. occur in the tune header.
    prelude: Vec<SequentialEntity>,

    /// The RepeatStructure stores the sequential entities.
    structures: Vec<RepeatStructure>,

    /// All non-sequential entities live in an a parallel structure.
    non_sequential_entities: Vec<NonSequentialEntity<'a>>,
}

impl<'a> Tune<'a> {
    pub fn new() -> Tune<'a> {
        Tune {
            prelude: vec![],
            structures: vec![],
            non_sequential_entities: vec![],
        }
    }
}

#[derive(Debug)]
struct Bar {
    /// A Bar has a number of parts (voices), each having a sequence of entities.
    parts: Vec<Vec<SequentialEntity>>,
}

#[derive(Debug)]
struct Section {
    opening_repeat: bool,
    /// A Section is made up of a sequence of bars.
    bars: Vec<Bar>,
    closing_repeat: bool,
}

impl Section {
    pub fn new() -> Section {
        Section {
            bars: vec![],
            opening_repeat: false,
            closing_repeat: false,
        }
    }
}

#[derive(Debug)]
struct RepeatStructure {
    /// Every tune is made of sections, even if only one.
    main_section: Section,

    /// Optional number of repeat bars.
    n_time_bars: Vec<Section>,
}

impl RepeatStructure {
    fn new() -> RepeatStructure {
        RepeatStructure {
            main_section: Section::new(),
            n_time_bars: vec![],
        }
    }
}

/// A SequentialEntity is something for which sequence is important.
/// This includes changes in metadata, which can happen out of alignment with the bar structure.
#[derive(Debug)]
enum SequentialEntity {
    Note(music::Note),

    // Metadata
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

    GuitarChord,
}

#[derive(Debug)]
enum NonSequentialEntity<'a> {
    /// A phrase mark spans from one sequential entity to another.
    PhraseMark(&'a SequentialEntity, &'a SequentialEntity),
}



/// Read from a Lexer and build a new AST.
pub fn read_from_lexer(lexer: l::Lexer) -> Tune {
    let mut result = Tune::new();

    // Prelude contains the entities that make up the header.
    let mut prelude_concluded = false;

    // The current sequence. Initially this is the prelude, but during a tune body this is a
    // buffer for the current bar's contents.
    let mut sequence: Vec<SequentialEntity> = vec![];

    // The current section.
    let mut section = Section::new();

    // The current structure
    let mut repeat_structure = RepeatStructure::new();

    // The base note length. This can change during the tune.
    let mut note_length = music::FractionalDuration(1, 4);

    for token in lexer {
        match token {

            l::LexResult::Error(_, offset, error) => (),

            // If there's a token we don't care about the context.
            l::LexResult::T(_, token) => {
                match token {
                    l::T::Terminal => (),
                    l::T::Newline => (),
                    l::T::Area(value) => sequence.push(SequentialEntity::Area(value)),
                    l::T::Book(value) => sequence.push(SequentialEntity::Book(value)),
                    l::T::Composer(value) => sequence.push(SequentialEntity::Composer(value)),
                    l::T::Discography(value) => sequence.push(SequentialEntity::Discography(value)),
                    l::T::Filename(value) => sequence.push(SequentialEntity::Filename(value)),
                    l::T::Group(value) => sequence.push(SequentialEntity::Group(value)),
                    l::T::History(value) => sequence.push(SequentialEntity::History(value)),
                    l::T::Information(value) => sequence.push(SequentialEntity::Information(value)),
                    l::T::Notes(value) => sequence.push(SequentialEntity::Notes(value)),
                    l::T::Origin(value) => sequence.push(SequentialEntity::Origin(value)),
                    l::T::Source(value) => sequence.push(SequentialEntity::Source(value)),
                    l::T::Title(value) => sequence.push(SequentialEntity::Title(value)),
                    l::T::Words(value) => sequence.push(SequentialEntity::Words(value)),
                    l::T::X(value) => sequence.push(SequentialEntity::X(value)),
                    l::T::Transcription(value) => {
                        sequence.push(SequentialEntity::Transcription(value))
                    }
                    l::T::Metre(numerator, denomenator) => {
                        sequence.push(SequentialEntity::Metre(numerator, denomenator))
                    }
                    l::T::KeySignature(pitch_class, mode) => {
                        sequence.push(SequentialEntity::KeySignature(pitch_class, mode));

                        // First time we hit a key signature, that signals that it's time to close
                        // the prelude and start a tune section.
                        if !prelude_concluded {
                            result.prelude = sequence;
                            sequence = vec![];
                            prelude_concluded = true;
                        }

                    }
                    l::T::DefaultNoteLength(new_note_length) => note_length = new_note_length,
                    l::T::Barline(barline) => {

                        if sequence.len() > 0 {
                            // A bar is made up of one or more sequences. If we've hit a barline,
                            // we've got to the last sequence of the bar.
                            let bar = Bar { parts: vec![sequence] };
                            section.bars.push(bar);
                        }

                        // And start a new sequence buffer.
                        sequence = vec![];

                        // If there's a repeat mark, this signals the end of a section.
                        // If not, continue with the section.
                        // TODO l::T::Barline should also represent n-time bars.
                        if barline.repeat_before || barline.repeat_after {
                            if barline.repeat_before {
                                section.closing_repeat = true;
                                repeat_structure.main_section = section;

                                section = Section::new();

                                result.structures.push(repeat_structure);

                                repeat_structure = RepeatStructure::new();
                            }

                            if barline.repeat_after {
                                section.opening_repeat = true;
                            }
                        }
                    }
                    l::T::Note(note) => {
                        sequence.push(SequentialEntity::Note(note.resolve_duration(note_length)))
                    }
                    l::T::BeamBreak => (),
                }
            }

        }
    }

    result
}
