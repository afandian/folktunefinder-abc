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

    // TODO matching on a reference and then cloning is deeply unsatisfactory!
    for token in lexer.collect_tokens().iter() {
        match token {
            &l::T::Newline => (),
            &l::T::Area(ref value) => sequence.push(SequentialEntity::Area(value.clone())),
            &l::T::Book(ref value) => sequence.push(SequentialEntity::Book(value.clone())),
            &l::T::Composer(ref value) => sequence.push(SequentialEntity::Composer(value.clone())),
            &l::T::Discography(ref value) => {
                sequence.push(SequentialEntity::Discography(value.clone()))
            }
            &l::T::Filename(ref value) => sequence.push(SequentialEntity::Filename(value.clone())),
            &l::T::Group(ref value) => sequence.push(SequentialEntity::Group(value.clone())),
            &l::T::History(ref value) => sequence.push(SequentialEntity::History(value.clone())),
            &l::T::Information(ref value) => {
                sequence.push(SequentialEntity::Information(value.clone()))
            }
            &l::T::Notes(ref value) => sequence.push(SequentialEntity::Notes(value.clone())),
            &l::T::Origin(ref value) => sequence.push(SequentialEntity::Origin(value.clone())),
            &l::T::Source(ref value) => sequence.push(SequentialEntity::Source(value.clone())),
            &l::T::Title(ref value) => sequence.push(SequentialEntity::Title(value.clone())),
            &l::T::Words(ref value) => sequence.push(SequentialEntity::Words(value.clone())),
            &l::T::X(ref value) => sequence.push(SequentialEntity::X(value.clone())),
            &l::T::Transcription(ref value) => {
                sequence.push(SequentialEntity::Transcription(value.clone()))
            }
            &l::T::Metre(ref numerator, ref denomenator) => {
                sequence.push(SequentialEntity::Metre(
                    numerator.clone(),
                    denomenator.clone(),
                ))
            }
            &l::T::KeySignature(ref pitch_class, ref mode) => {
                sequence.push(SequentialEntity::KeySignature(
                    pitch_class.clone(),
                    mode.clone(),
                ));

                // K marks the end of the prelude.
                if !finished_prelude {
                    tune.prelude = sequence;
                    finished_prelude = true;
                    sequence = vec![];
                }
            }
            &l::T::DefaultNoteLength(new_note_length) => note_length = new_note_length,
            // &l::T::Barline(ref barline) => {

            //     // End of a bar, flush the sequence, if there is one.
            //     if sequence.len() > 0 {
            //         bar.sequences.push(sequence);
            //         sequence = vec![];
            //     }


            //     // Where should we put this bar?
            //     match section_mode {
            //         SectionMode::Main => {
            //             section.main.push(bar);
            //         }
            //         SectionMode::NTimeBar(_) => {
            //             // TODO DISCARDING N!
            //             // There will always be a last due to the section_mode change below.
            //             section.n_time_bars.last_mut().unwrap().push(bar);
            //         }
            //     }
            //     bar = Bar::new();

            //     // This signals the start of an n-time bar.
            //     if let Some(n_time) = barline.n_time {
            //         section_mode = SectionMode::NTimeBar(n_time);
            //         section.n_time_bars.push(vec![]);
            //     } else {
            //         // Is this starting a new repeated section?
            //         // Don't need to record the repeat mark, it's implicit in the structure.
            //         // Also it's often optional anyway.
            //         if barline.repeat_after {
            //             section_mode = SectionMode::Main;
            //         }

            //         // Is this closing a repeated section?
            //         if barline.repeat_before {
            //             section.repeat = true;
            //             tune.sections.push(section);
            //             section = Section::new();
            //             section_mode = SectionMode::Main;
            //         }

            //         if !barline.single {
            //             tune.sections.push(section);
            //             section = Section::new();
            //             section_mode = SectionMode::Main;
            //         }
            //     }



            // }
            &l::T::Note(ref note) => {
                sequence.push(SequentialEntity::Note(note.resolve_duration(note_length)))
            }
            &l::T::BeamBreak => (),

            _ => println!("TODO! Unhandled")
        }


    }

    tune
}
