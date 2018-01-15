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
    pub prelude: Vec<SequentialEntity>,

    /// The RepeatStructure stores the sequential entities.
    pub sections: Vec<Section>,

    /// All non-sequential entities live in an a parallel structure.
    pub non_sequential_entities: Vec<NonSequentialEntity<'a>>,
}

impl<'a> Tune<'a> {
    pub fn new() -> Tune<'a> {
        Tune {
            prelude: vec![],
            sections: vec![],
            non_sequential_entities: vec![],
        }
    }
}

#[derive(Debug)]
pub struct Bar {
    /// A Bar has a number of parts (voices), each having a sequence of entities.
    sequences: Vec<Vec<SequentialEntity>>,
}

impl Bar {
    pub fn new() -> Bar {
        Bar { sequences: vec![] }
    }
}

#[derive(Debug)]
pub struct Section {
    pub repeat: bool,

    pub main: Vec<Bar>,
    pub n_time_bars: Vec<Vec<Bar>>,
}

impl Section {
    pub fn new() -> Section {
        Section {
            repeat: false,
            main: vec![],
            n_time_bars: vec![],
        }
    }
}

/// A SequentialEntity is something for which sequence is important.
/// This includes changes in metadata, which can happen out of alignment with the bar structure.
#[derive(Debug)]
pub enum SequentialEntity {
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
pub enum NonSequentialEntity<'a> {
    /// A phrase mark spans from one sequential entity to another.
    PhraseMark(&'a SequentialEntity, &'a SequentialEntity),
}

#[derive(Debug)]
pub enum SectionMode {
    Main,
    NTimeBar(u32),
}


/// Read from a Lexer and build a new AST.
pub fn read_from_lexer(lexer: l::Lexer) -> Tune {
    let mut tune = Tune::new();

    let mut finished_prelude = false;



    let mut bar = Bar::new();
    let mut sequence = vec![];
    // let mut bars: Vec<Bar> = vec![];

    let mut section = Section::new();




    let mut section_mode = SectionMode::Main;

    // The base note length. This can change during the tune.
    let mut note_length = music::FractionalDuration(1, 4);

    for token in lexer {
        match token {

            l::LexResult::Error(_, _, _) => (),

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

                        // K marks the end of the prelude.
                        if !finished_prelude {
                            tune.prelude = sequence;
                            finished_prelude = true;
                            sequence = vec![];
                        }
                    }
                    l::T::DefaultNoteLength(new_note_length) => note_length = new_note_length,
                    l::T::Barline(barline) => {

                        // End of a bar, flush the sequence, if there is one.
                        if sequence.len() > 0 {
                            bar.sequences.push(sequence);
                            sequence = vec![];
                        }


                        // Where should we put this bar?
                        match section_mode {
                            SectionMode::Main => {
                                section.main.push(bar);
                            }
                            SectionMode::NTimeBar(_) => {
                                // TODO DISCARDING N!
                                // There will always be a last due to the section_mode change below.
                                section.n_time_bars.last_mut().unwrap().push(bar);
                            }
                        }
                        bar = Bar::new();

                        // This signals the start of an n-time bar.
                        if let Some(n_time) = barline.n_time {
                            section_mode = SectionMode::NTimeBar(n_time);
                            section.n_time_bars.push(vec![]);
                        } else {
                            // Is this starting a new repeated section?
                            // Don't need to record the repeat mark, it's implicit in the structure.
                            // Also it's often optional anyway.
                            if barline.repeat_after {
                                section_mode = SectionMode::Main;
                            }

                            // Is this closing a repeated section?
                            if barline.repeat_before {
                                section.repeat = true;
                                tune.sections.push(section);
                                section = Section::new();
                                section_mode = SectionMode::Main;
                            }

                            if !barline.single {
                                tune.sections.push(section);
                                section = Section::new();
                                section_mode = SectionMode::Main;
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

    tune
}
