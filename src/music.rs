
#[derive(Debug, PartialEq, PartialOrd, Clone)]
pub enum DiatonicPitchClass {
    A,
    B,
    C,
    D,
    E,
    F,
    G,
}

#[derive(Debug, PartialEq, PartialOrd, Clone)]
pub enum Accidental {
    Sharp,
    Flat,
    Natural,
    DoubleSharp,
    DoubleFlat,
}

/// Musical Mode
/// Some of these are synonyms, but we want to record what was written.
#[derive(Debug, PartialEq, PartialOrd, Clone)]
pub enum Mode {
    Natural,

    Lydian,
    Ionian,
    Mixolydian,
    Dorian,
    Aeolian,
    Phrygian,
    Locrian,

    Major,
    Minor,
}


#[derive(Debug, PartialEq, PartialOrd, Clone)]
pub struct PitchClass(pub DiatonicPitchClass, pub Option<Accidental>);

#[derive(Debug, PartialEq, PartialOrd, Clone)]
pub struct Pitch(
    pub PitchClass,
    /// Octave
    pub i16
);


#[derive(Debug, PartialEq, PartialOrd, Clone)]
pub struct Barline {
    pub repeat_before: bool,
    pub single: bool,
    pub repeat_after: bool,
}

/// A duration as a fraction of the default duration.
#[derive(Debug, PartialEq, PartialOrd, Clone)]
pub struct FractionalDuration(pub u32, pub u32);

#[derive(Debug, PartialEq, PartialOrd, Clone)]
pub struct Note(pub Pitch, pub FractionalDuration);
