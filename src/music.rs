const NOTES_IN_SCALE: i16 = 7;

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

impl DiatonicPitchClass {
    pub fn to_degree(&self) -> i16 {
        match self {
            &DiatonicPitchClass::C => 0,
            &DiatonicPitchClass::D => 1,
            &DiatonicPitchClass::E => 2,
            &DiatonicPitchClass::F => 3,
            &DiatonicPitchClass::G => 4,
            &DiatonicPitchClass::A => 5,
            &DiatonicPitchClass::B => 6,
        }
    }
}

#[derive(Debug, PartialEq, PartialOrd, Clone)]
pub enum Accidental {
    Sharp,
    Flat,
    Natural,
    DoubleSharp,
    DoubleFlat,
}

impl Accidental {
    pub fn semitones(&self) -> i16 {
        match self {
            &Accidental::Sharp => 1,
            &Accidental::Flat => -1,
            &Accidental::Natural => 0,
            &Accidental::DoubleSharp => 2,
            &Accidental::DoubleFlat => -2,
        }
    }
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
pub enum ClefShape {
    Treble,
}

impl ClefShape {
    /// What pitch does this shape represent?
    pub fn pitch(&self) -> PitchClass {
        match self {
            Treble => PitchClass {
                diatonic_pitch_class: DiatonicPitchClass::G,
                accidental: None,
            },
        }
    }
}

#[derive(Debug, PartialEq, PartialOrd, Clone)]
pub struct Clef {
    shape: ClefShape,
    // Position on stave relative to middle line.
    centre: i16,
    pitch: PitchClass,
}

impl Clef {
    /// Construct a treble clef.
    pub fn treble() -> Clef {
        Clef {
            shape: ClefShape::Treble,
            centre: 3,
            pitch: PitchClass {
                diatonic_pitch_class: DiatonicPitchClass::G,
                accidental: None,
            },
        }
    }
}

#[derive(Debug, PartialEq, PartialOrd, Clone)]
pub struct PitchClass {
    pub diatonic_pitch_class: DiatonicPitchClass,
    pub accidental: Option<Accidental>,
}


/// Interval as number of tones and an accidental.
/// Note that "unison" is expressed as "1" but here as 0.
#[derive(Debug, PartialEq, PartialOrd, Clone)]
pub struct Interval {
    /// Interval
    pitch_classes: i16,
    /// Accidental
    accidental_semitones: i16,
}

#[derive(Debug, PartialEq, PartialOrd, Clone)]
pub struct Pitch {
    pub pitch_class: PitchClass,
    /// Octave
    pub octave: i16,
}

impl Pitch {
    // How many diatonic degrees between this note and another.
    // Note that if this occurs in a key signature, the key signature must be applied first!
    pub fn interval_to(&self, other: Pitch) -> Interval {
        let degrees = (other.pitch_class.diatonic_pitch_class.to_degree() +
                           NOTES_IN_SCALE * other.octave) -
            (self.pitch_class.diatonic_pitch_class.to_degree() + NOTES_IN_SCALE * self.octave);

        let accidental = match other.pitch_class.accidental {
            None => 0,
            Some(ref accidental) => accidental.semitones(),
        } -
            match self.pitch_class.accidental {
                None => 0,
                Some(ref accidental) => accidental.semitones(),
            };

        Interval {
            pitch_classes: degrees,
            accidental_semitones: accidental,
        }
    }
}


#[derive(Debug, PartialEq, PartialOrd, Clone)]
pub struct Barline {
    pub repeat_before: bool,
    pub single: bool,
    pub repeat_after: bool,
    pub n_time: Option<u32>,
}

/// A duration as a fraction of the default duration.
#[derive(Debug, PartialEq, PartialOrd, Clone, Copy)]
pub struct FractionalDuration(pub u32, pub u32);

impl FractionalDuration {
    /// Multiply this fractional duration by another.
    /// Used to resolve a duration against a standard duration.
    pub fn multiply(self, other: FractionalDuration) -> FractionalDuration {

        let vulgar = FractionalDuration(self.0 * other.0, self.1 * other.1);

        let max = u32::max(vulgar.0, vulgar.1);
        for i in (1..max).rev() {
            if (vulgar.0 % i == 0) && (vulgar.1 % i) == 0 {
                return FractionalDuration(vulgar.0 / i, vulgar.1 / i);
            }
        }
        return vulgar;
    }
}

#[derive(Debug, PartialEq, PartialOrd, Clone)]
pub struct Note(pub Pitch, pub FractionalDuration);

impl Note {
    /// Adjust this note's duration by mutiplying by a base.
    pub fn resolve_duration(self, base_duration: FractionalDuration) -> Note {
        Note(self.0, self.1.multiply(base_duration))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fractional_duration_multiply() {
        assert_eq!(
            FractionalDuration(1, 1).multiply(FractionalDuration(1, 4)),
            FractionalDuration(1, 4),
            "Resolving duration of 1 in 1/4 gives 1/4"
        );

        assert_eq!(
            FractionalDuration(2, 1).multiply(FractionalDuration(1, 4)),
            FractionalDuration(1, 2),
            "Resolving duration of 1 in 1/4 gives simplified 1/2"
        );

        assert_eq!(
            FractionalDuration(3, 1).multiply(FractionalDuration(1, 4)),
            FractionalDuration(3, 4),
            "Resolving dotted crotchet gives dotted crotchet (can't simplify further)."
        );

        assert_eq!(
            FractionalDuration(1, 8).multiply(FractionalDuration(1, 2)),
            FractionalDuration(1, 2).multiply(FractionalDuration(1, 8)),
            "Multiply is commutative."
        );
    }

    #[test]
    fn pitch_minus_as_degrees_test() {
        assert_eq!(
            Pitch {
                pitch_class: PitchClass {
                    diatonic_pitch_class: DiatonicPitchClass::G,
                    accidental: None,
                },
                octave: 0,
            }.interval_to(Pitch {
                pitch_class: PitchClass {
                    diatonic_pitch_class: DiatonicPitchClass::G,
                    accidental: None,
                },
                octave: 0,
            }),
            Interval {
                pitch_classes: 0,
                accidental_semitones: 0,
            },
            "Note should have zero distance to itself"
        );

        assert_eq!(
            Pitch {
                pitch_class: PitchClass {
                    diatonic_pitch_class: DiatonicPitchClass::G,
                    accidental: None,
                },
                octave: 0,
            }.interval_to(Pitch {
                pitch_class: PitchClass {
                    diatonic_pitch_class: DiatonicPitchClass::G,
                    accidental: None,
                },
                octave: 1,
            }),
            Interval {
                pitch_classes: 7,
                accidental_semitones: 0,
            },
            "Note should have 7 distance to itself in the next octave"
        );

        assert_eq!(
            Pitch {
                pitch_class: PitchClass {
                    diatonic_pitch_class: DiatonicPitchClass::G,
                    accidental: None,
                },
                octave: 0,
            }.interval_to(Pitch {
                pitch_class: PitchClass {
                    diatonic_pitch_class: DiatonicPitchClass::G,
                    accidental: None,
                },
                octave: -1,
            }),
            Interval {
                pitch_classes: -7,
                accidental_semitones: 0,
            },
            "Note should have -7 distance to itself in the previous octave"
        );

        assert_eq!(
            Pitch {
                pitch_class: PitchClass {
                    diatonic_pitch_class: DiatonicPitchClass::G,
                    accidental: None,
                },
                octave: 0,
            }.interval_to(Pitch {
                pitch_class: PitchClass {
                    diatonic_pitch_class: DiatonicPitchClass::A,
                    accidental: Some(Accidental::Sharp),
                },
                octave: 0,
            }),
            Interval {
                pitch_classes: 1,
                accidental_semitones: 1,
            },
            "Augmented first."
        );


    }

}
