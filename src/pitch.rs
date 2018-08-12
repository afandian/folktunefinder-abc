use abc_lexer as l;
use music;
use std::f32;
use tune_ast_three;

// Convert to a monophonic sequence of pitches as MIDI pitch.
// TODO Currently ignores repeat bars, key signature and mode.
pub fn build_pitch_sequence(ast: &tune_ast_three::Tune) -> Vec<u8> {
    let mut result = vec![];

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
                    let music::Note(pitch, duration) = note;
                    let midi_pitch = pitch.midi_pitch();
                    result.push(midi_pitch);
                }

                _ => {}
            }
        }
    }

    result
}

pub fn pitch_seq_to_intervals(pitches: &Vec<u8>) -> Vec<i16> {
    let mut result = Vec::with_capacity(pitches.len());

    let mut last: i16 = 0;
    let mut first = true;
    for pitch in pitches.iter() {
        if (first) {
            first = false;
        } else {
            let interval = (*pitch as i16) - last;
            result.push(interval);
        }

        last = *pitch as i16;
    }

    result
}

// Number of chromatic pitches either size of zero to take.
const HISTOGRAM_SIZE: usize = 12;

// Resulting size of histogram, including zero.
pub const HISTOGRAM_WIDTH: usize = HISTOGRAM_SIZE + HISTOGRAM_SIZE + 2;

pub fn build_interval_histogram(pitches: &Vec<i16>) -> [f32; HISTOGRAM_WIDTH] {
    let mut histogram = [0.0; HISTOGRAM_WIDTH];

    let max = HISTOGRAM_WIDTH as i16;
    let min = HISTOGRAM_WIDTH as i16 * -1;

    for interval in pitches.iter() {
        // Clamp to range, saturating at each end.
        let i = i16::min(
            i16::max(*interval + HISTOGRAM_SIZE as i16, 0),
            (HISTOGRAM_WIDTH - 1) as i16,
        );
        histogram[i as usize] += 1.0;
    }

    if pitches.len() > 0 {
        let count = pitches.len() as f32;
        for i in 0..HISTOGRAM_WIDTH {
            histogram[i] /= count;
        }
    }

    histogram
}

pub fn sim_interval_histogram(a: &[f32; HISTOGRAM_WIDTH], b: &[f32; HISTOGRAM_WIDTH]) -> f32 {
    let mut result = 0.0;
    for i in 0..HISTOGRAM_WIDTH {
        result += (b[i] - a[i]) * (b[i] - a[i]);
    }

    f32::sqrt(result)
}
