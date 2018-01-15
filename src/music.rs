
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
}
