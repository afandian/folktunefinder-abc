pub const NOTES_IN_SCALE: i16 = 7;

#[derive(Debug, PartialEq, PartialOrd, Clone, Copy)]
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

    pub fn to_chromatic(&self) -> u8 {
        match self {
            &DiatonicPitchClass::C => 0,
            &DiatonicPitchClass::D => 2,
            &DiatonicPitchClass::E => 4,
            &DiatonicPitchClass::F => 5,
            &DiatonicPitchClass::G => 7,
            &DiatonicPitchClass::A => 9,
            &DiatonicPitchClass::B => 11,
        }
    }

    pub fn to_string(&self) -> String {
        match self {
            &DiatonicPitchClass::C => "C".to_string(),
            &DiatonicPitchClass::D => "D".to_string(),
            &DiatonicPitchClass::E => "E".to_string(),
            &DiatonicPitchClass::F => "F".to_string(),
            &DiatonicPitchClass::G => "G".to_string(),
            &DiatonicPitchClass::A => "A".to_string(),
            &DiatonicPitchClass::B => "B".to_string(),
        }
    }
}

#[derive(Debug, PartialEq, PartialOrd, Clone, Copy)]
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

    pub fn to_string(&self) -> String {
        match self {
            &Accidental::Sharp => "♯".to_string(),
            &Accidental::Flat => "♭".to_string(),
            &Accidental::Natural => "♮".to_string(),
            &Accidental::DoubleSharp => "𝄪".to_string(),
            &Accidental::DoubleFlat => "𝄫".to_string(),
        }
    }
}

/// Musical Mode
/// Some of these are synonyms, but we want to record what was written.
#[derive(Debug, PartialEq, PartialOrd, Clone, Copy)]
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

impl Mode {
    pub fn to_string(&self) -> String {
        match self {
            &Mode::Natural => "Natural".to_string(),
            &Mode::Lydian => "Lydian".to_string(),
            &Mode::Ionian => "Ionian".to_string(),
            &Mode::Mixolydian => "Mixolydian".to_string(),
            &Mode::Dorian => "Dorian".to_string(),
            &Mode::Aeolian => "Aeolian".to_string(),
            &Mode::Phrygian => "Phrygian".to_string(),
            &Mode::Locrian => "Locrian".to_string(),
            &Mode::Major => "Major".to_string(),
            &Mode::Minor => "Minor".to_string(),
        }
    }
}

#[derive(Debug, PartialEq, PartialOrd, Clone, Copy)]
pub enum ClefShape {
    Treble,
}

impl ClefShape {
    /// What pitch does this shape represent?
    pub fn pitch(&self) -> PitchClass {
        match self {
            ClefShape::Treble => PitchClass {
                diatonic_pitch_class: DiatonicPitchClass::G,
                accidental: None,
            },
        }
    }
}

#[derive(Debug, PartialEq, PartialOrd, Clone, Copy)]
pub struct Clef {
    pub shape: ClefShape,
    // Position on stave relative to middle line.
    pub centre: i32,
    pub pitch: Pitch,
}

impl Clef {
    /// Construct a treble clef.
    pub fn treble() -> Clef {
        Clef {
            shape: ClefShape::Treble,
            centre: 2,
            pitch: Pitch {
                pitch_class: PitchClass {
                    diatonic_pitch_class: DiatonicPitchClass::G,
                    accidental: None,
                },
                octave: 0,
            },
        }
    }
}

#[derive(Debug, PartialEq, PartialOrd, Clone, Copy)]
pub struct PitchClass {
    pub diatonic_pitch_class: DiatonicPitchClass,
    pub accidental: Option<Accidental>,
}

impl PitchClass {
    pub fn to_chromatic(&self) -> u8 {
        (self.diatonic_pitch_class.to_chromatic() as i16 + match self.accidental {
            Some(accidental) => accidental.semitones(),
            _ => 0,
        }) as u8
    }

    pub fn to_string(&self) -> String {
        format!(
            "{}{}",
            self.diatonic_pitch_class.to_string(),
            match self.accidental {
                Some(accidental) => accidental.to_string(),
                None => "".to_string(),
            }
        )
    }
}

/// Interval as number of tones and an accidental.
/// Note that "unison" is expressed as "1" but here as 0.
#[derive(Debug, PartialEq, PartialOrd, Clone, Copy)]
pub struct Interval {
    /// Interval
    pub pitch_classes: i32,
    /// Accidental
    pub accidental_semitones: i16,
}

#[derive(Debug, PartialEq, PartialOrd, Clone, Copy)]
pub struct Pitch {
    pub pitch_class: PitchClass,
    /// Octave
    pub octave: i16,
}

impl Pitch {
    // How many diatonic degrees between this note and another.
    // Note that if this occurs in a key signature, the key signature must be applied first!
    pub fn interval_to(&self, other: Pitch) -> Interval {
        let degrees = (other.pitch_class.diatonic_pitch_class.to_degree()
            + NOTES_IN_SCALE * other.octave)
            - (self.pitch_class.diatonic_pitch_class.to_degree() + NOTES_IN_SCALE * self.octave);

        let accidental = match other.pitch_class.accidental {
            None => 0,
            Some(ref accidental) => accidental.semitones(),
        } - match self.pitch_class.accidental {
            None => 0,
            Some(ref accidental) => accidental.semitones(),
        };

        Interval {
            pitch_classes: degrees as i32,
            accidental_semitones: accidental,
        }
    }

    // TODO no key signature or mode yet!
    pub fn midi_pitch(&self) -> u8 {
        let diatonic_pitch = self.pitch_class.to_chromatic();
        (diatonic_pitch as i16 + (self.octave * 12) + 60) as u8
    }
}

/// Time signature
#[derive(Debug, PartialEq, PartialOrd, Clone, Copy)]
pub struct Metre(pub u32, pub u32);

impl Metre {
    pub fn to_string(&self) -> String {
        format!("{}/{}", self.0, self.1)
    }
}

/// The duration class of a notehead, i.e. its shape.
#[derive(Debug, Copy, Clone, PartialEq, PartialOrd)]
pub enum DurationClass {
    Semibreve,
    Minim,
    Crotchet,
    Quaver,
    Semiquaver,
    Demisemiquaver,
}

// All duration classes, in order of duration.
const DURATION_CLASSES: &[DurationClass] = &[
    DurationClass::Semibreve,
    DurationClass::Minim,
    DurationClass::Crotchet,
    DurationClass::Quaver,
    DurationClass::Semiquaver,
    DurationClass::Demisemiquaver,
];

impl DurationClass {
    fn duration(&self) -> FractionalDuration {
        match self {
            &DurationClass::Semibreve => FractionalDuration(1, 1),
            &DurationClass::Minim => FractionalDuration(1, 2),
            &DurationClass::Crotchet => FractionalDuration(1, 4),
            &DurationClass::Quaver => FractionalDuration(1, 8),
            &DurationClass::Semiquaver => FractionalDuration(1, 16),
            &DurationClass::Demisemiquaver => FractionalDuration(1, 32),
        }
    }

    // How many beams / tails does this duration render as?
    pub fn beams(&self) -> u32 {
        match self {
            &DurationClass::Semibreve => 0,
            &DurationClass::Minim => 0,
            &DurationClass::Crotchet => 0,
            &DurationClass::Quaver => 1,
            &DurationClass::Semiquaver => 2,
            &DurationClass::Demisemiquaver => 3,
        }
    }
}

/// Represent a duration per notation.
#[derive(Debug, PartialEq, Copy, Clone, PartialOrd)]
pub struct DurationGlyph {
    pub shape: DurationClass,
    pub dots: u32,
}

/// A duration as a fraction of the default duration.
#[derive(Debug, PartialEq, PartialOrd, Clone, Copy)]
pub struct FractionalDuration(pub u32, pub u32);

impl FractionalDuration {
    /// Multiply this fractional duration by another.
    /// Used to resolve a duration against a standard duration.
    pub fn multiply(self, other: FractionalDuration) -> FractionalDuration {
        let vulgar = FractionalDuration(self.0 * other.0, self.1 * other.1);

        return vulgar.reduce();
    }

    pub fn subtract(self, other: FractionalDuration) -> FractionalDuration {
        let self_numerator = self.0 * other.1;
        let other_numerator = other.0 * self.1;
        let denomenator = self.1 * other.1;

        FractionalDuration(self_numerator - other_numerator, denomenator).reduce()
    }

    /// Reduce this fraction to its simplest form.
    pub fn reduce(self) -> FractionalDuration {
        let max = u32::max(self.0, self.1);
        for i in (1..max + 1).rev() {
            if (self.0 % i == 0) && (self.1 % i) == 0 {
                return FractionalDuration(self.0 / i, self.1 / i);
            }
        }

        self
    }

    /// Is this duration greater than the other one?
    /// TODO Implement PartialOrd properly!
    pub fn gte(&self, other: &FractionalDuration) -> bool {
        let self_numerator = self.0 * other.1;
        let other_numerator = other.0 * self.1;
        self_numerator >= other_numerator
    }

    /// Transform this duration into a notehead glyph.
    /// i.e. "3/2" becomes "dotted crotchet".
    /// TODO in future this may be represented as a sequence of tied glyphs
    /// for complicted durations.
    pub fn to_glyph(&self) -> Option<DurationGlyph> {
        const MAX_DOTS: u32 = 4;

        // Start with self's duration, keep chipping away until there's nothing left to represent.
        let mut this = *self;

        let mut result = None;

        // Try each top level duration class first.
        for duration_class in DURATION_CLASSES.iter() {
            // When there's nothing left to represent, stop there.
            if this.reduce() == FractionalDuration(0, 1) {
                break;
            }

            let mut duration = duration_class.duration();
            let mut num_dots = 0;

            // It is possible to represent self duration using this duration class.
            if this.gte(&duration) {
                for _ in 0..MAX_DOTS + 1 {
                    this = this.subtract(duration);

                    if this.reduce() == FractionalDuration(0, 1) {
                        break;
                    }

                    // Half the duration to correspond to another dot.
                    duration = duration.multiply(FractionalDuration(1, 2));
                    num_dots += 1;
                }

                result = Some(DurationGlyph {
                    shape: *duration_class,
                    dots: num_dots,
                });
            }
        }

        result
    }
}

#[derive(Debug, PartialEq, PartialOrd, Clone)]
pub struct Note(pub Pitch, pub FractionalDuration);

impl Note {
    /// Adjust this note's duration by mutiplying by a base.
    pub fn resolve_duration(&self, base_duration: FractionalDuration) -> Note {
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
    fn fractional_duration_multiply_test() {
        assert_eq!(FractionalDuration(1, 1).reduce(), FractionalDuration(1, 1));
        assert_eq!(FractionalDuration(2, 2).reduce(), FractionalDuration(1, 1));
        assert_eq!(
            FractionalDuration(16, 16).reduce(),
            FractionalDuration(1, 1)
        );
        assert_eq!(FractionalDuration(2, 4).reduce(), FractionalDuration(1, 2));
        assert_eq!(FractionalDuration(2, 6).reduce(), FractionalDuration(1, 3));
    }

    #[test]
    fn duration_to_glyph_simple_test() {
        // Simple durations.
        assert_eq!(
            FractionalDuration(1, 1).to_glyph(),
            Some(DurationGlyph {
                shape: DurationClass::Semibreve,
                dots: 0,
            })
        );
        assert_eq!(
            FractionalDuration(1, 2).to_glyph(),
            Some(DurationGlyph {
                shape: DurationClass::Minim,
                dots: 0,
            })
        );
        assert_eq!(
            FractionalDuration(1, 4).to_glyph(),
            Some(DurationGlyph {
                shape: DurationClass::Crotchet,
                dots: 0,
            })
        );
        assert_eq!(
            FractionalDuration(1, 8).to_glyph(),
            Some(DurationGlyph {
                shape: DurationClass::Quaver,
                dots: 0,
            })
        );
        assert_eq!(
            FractionalDuration(1, 16).to_glyph(),
            Some(DurationGlyph {
                shape: DurationClass::Semiquaver,
                dots: 0,
            })
        );
        assert_eq!(
            FractionalDuration(1, 32).to_glyph(),
            Some(DurationGlyph {
                shape: DurationClass::Demisemiquaver,
                dots: 0,
            })
        );
    }

    #[test]
    fn duration_to_glyph_dotted_test() {
        assert_eq!(
            FractionalDuration(3, 2).to_glyph(),
            Some(DurationGlyph {
                shape: DurationClass::Semibreve,
                dots: 1,
            })
        );
        assert_eq!(
            FractionalDuration(3, 4).to_glyph(),
            Some(DurationGlyph {
                shape: DurationClass::Minim,
                dots: 1,
            })
        );
        assert_eq!(
            FractionalDuration(3, 8).to_glyph(),
            Some(DurationGlyph {
                shape: DurationClass::Crotchet,
                dots: 1,
            })
        );
        assert_eq!(
            FractionalDuration(3, 16).to_glyph(),
            Some(DurationGlyph {
                shape: DurationClass::Quaver,
                dots: 1,
            })
        );
        assert_eq!(
            FractionalDuration(3, 32).to_glyph(),
            Some(DurationGlyph {
                shape: DurationClass::Semiquaver,
                dots: 1,
            })
        );
        assert_eq!(
            FractionalDuration(3, 64).to_glyph(),
            Some(DurationGlyph {
                shape: DurationClass::Demisemiquaver,
                dots: 1,
            })
        );

        // Two dots
        assert_eq!(
            FractionalDuration(7, 4).to_glyph(),
            Some(DurationGlyph {
                shape: DurationClass::Semibreve,
                dots: 2,
            })
        );
        assert_eq!(
            FractionalDuration(7, 8).to_glyph(),
            Some(DurationGlyph {
                shape: DurationClass::Minim,
                dots: 2,
            })
        );
        assert_eq!(
            FractionalDuration(7, 16).to_glyph(),
            Some(DurationGlyph {
                shape: DurationClass::Crotchet,
                dots: 2,
            })
        );
        assert_eq!(
            FractionalDuration(7, 32).to_glyph(),
            Some(DurationGlyph {
                shape: DurationClass::Quaver,
                dots: 2,
            })
        );

        assert_eq!(
            FractionalDuration(7, 64).to_glyph(),
            Some(DurationGlyph {
                shape: DurationClass::Semiquaver,
                dots: 2,
            })
        );

        // A double-dotted semiquaver should be enough for anyone.
        assert_eq!(
            FractionalDuration(7, 128).to_glyph(),
            Some(DurationGlyph {
                shape: DurationClass::Demisemiquaver,
                dots: 2,
            })
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

    #[test]
    fn midi_pitch_test() {
        assert_eq!(
            Pitch {
                pitch_class: PitchClass {
                    diatonic_pitch_class: DiatonicPitchClass::C,
                    accidental: None,
                },
                octave: 0,
            }.midi_pitch(),
            60,
            "Middle C"
        );

        assert_eq!(
            Pitch {
                pitch_class: PitchClass {
                    diatonic_pitch_class: DiatonicPitchClass::C,
                    accidental: None,
                },
                octave: 1,
            }.midi_pitch(),
            72,
            "Octave C"
        );

        assert_eq!(
            Pitch {
                pitch_class: PitchClass {
                    diatonic_pitch_class: DiatonicPitchClass::C,
                    accidental: None,
                },
                octave: -1,
            }.midi_pitch(),
            48,
            "Low octave C"
        );

        assert_eq!(
            Pitch {
                pitch_class: PitchClass {
                    diatonic_pitch_class: DiatonicPitchClass::C,
                    accidental: Some(Accidental::Sharp),
                },
                octave: 0,
            }.midi_pitch(),
            61,
            "Middle C#"
        );

        assert_eq!(
            Pitch {
                pitch_class: PitchClass {
                    diatonic_pitch_class: DiatonicPitchClass::C,
                    accidental: Some(Accidental::Sharp),
                },
                octave: 1,
            }.midi_pitch(),
            73,
            "Octave C#"
        );

        assert_eq!(
            Pitch {
                pitch_class: PitchClass {
                    diatonic_pitch_class: DiatonicPitchClass::C,
                    accidental: Some(Accidental::Sharp),
                },
                octave: -1,
            }.midi_pitch(),
            49,
            "Low octave C#"
        );
    }

}
