use abc_lexer as l;
use music;
use std::f32;
use tune_ast_three;

pub struct PitchSequence {
    pub pitches: Vec<u8>,
}

impl PitchSequence {
    // Convert to a monophonic sequence of pitches as MIDI pitch.
    // TODO Currently ignores repeat bars, key signature and mode.
    pub fn from_ast(ast: &tune_ast_three::Tune) -> PitchSequence {
        let mut pitches = vec![];

        let mut key_signature = l::T::KeySignature(
            music::PitchClass {
                diatonic_pitch_class: music::DiatonicPitchClass::C,
                accidental: None,
            },
            music::Mode::Major,
        );

        for ref token in ast.prelude.iter() {
            match *token {
                l::T::KeySignature(pitch_class, mode) => {
                    key_signature = l::T::KeySignature(*pitch_class, *mode)
                }
                _ => {}
            }
        }

        for ref voice in ast.voices.iter() {
            for ref token in voice.iter() {
                match token {
                    l::T::Note(note) => {
                        // TODO extras like accidentals etc.
                        let music::Note(pitch, _duration) = note;
                        let midi_pitch = pitch.midi_pitch();
                        pitches.push(midi_pitch);
                    }

                    _ => {}
                }
            }
        }

        PitchSequence { pitches }
    }

    pub fn from_pitches(pitches: &Vec<u8>) -> PitchSequence {
        PitchSequence {
            pitches: pitches.clone(),
        }
    }
}

pub struct IntervalSequence {
    pub intervals: Vec<i16>,
}

impl IntervalSequence {
    pub fn from_pitch_sequence(pitches: &PitchSequence) -> IntervalSequence {
        let mut intervals = Vec::with_capacity(pitches.pitches.len());

        let mut last: i16 = 0;
        let mut first = true;
        for pitch in pitches.pitches.iter() {
            if first {
                first = false;
            } else {
                let interval = (*pitch as i16) - last;
                intervals.push(interval);
            }

            last = *pitch as i16;
        }

        IntervalSequence { intervals }
    }
}

// Number of chromatic pitches either size of zero to take.
const HISTOGRAM_SIZE: usize = 12;

// Resulting size of histogram, including zero.
pub const HISTOGRAM_WIDTH: usize = HISTOGRAM_SIZE + HISTOGRAM_SIZE + 2;

pub struct IntervalHistogram {
    pub histogram: [f32; HISTOGRAM_WIDTH],
}

impl IntervalHistogram {
    pub fn from_interval_seq(intervals: &IntervalSequence) -> IntervalHistogram {
        let mut histogram = [0.0; HISTOGRAM_WIDTH];

        let _max = HISTOGRAM_WIDTH as i16;
        let _min = HISTOGRAM_WIDTH as i16 * -1;

        for interval in intervals.intervals.iter() {
            // Clamp to range, saturating at each end.
            let i = i16::min(
                i16::max(*interval + HISTOGRAM_SIZE as i16, 0),
                (HISTOGRAM_WIDTH - 1) as i16,
            );
            histogram[i as usize] += 1.0;
        }

        if intervals.intervals.len() > 0 {
            let count = intervals.intervals.len() as f32;
            for i in 0..HISTOGRAM_WIDTH {
                histogram[i] /= count;
            }
        }

        IntervalHistogram { histogram }
    }

    pub fn sim(&self, other: &IntervalHistogram) -> f32 {
        let mut result = 0.0;
        for i in 0..HISTOGRAM_WIDTH {
            result +=
                (other.histogram[i] - self.histogram[i]) * (other.histogram[i] - self.histogram[i]);
        }

        f32::sqrt(result)
    }
}
