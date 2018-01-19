///! ABC Lexer
///! Transform strings of ABC into a sequence of lexed tokens.
///! This accepts a String and returns newly allocated strings that have an independent lifetime to
///! the supplied string.
///! When lex_* and read_* functions return errors, they should leave the context in the most
///! helpful state so that the next token has a good chance at understanding it.
///! e.g. don't bomb out half way through the time signature.
///! lex_functions are relatively context-free and return a top-level token wrapped in a LexResult.
///! They are called in a context where the token is expected, and raise an error when an unexpected
///! character was found.
///! read_functions are helpers, often represent optional branches, and generally return an Option.
///!  They are called speculatively, and simply return an option.
///! Context is a lightweight immutable pointer into a char slice. There's heavy (hopefully
///! sensible) use of shadowing / rebinding of 'ctx' variables, so check the scope!

use std::fmt;
use music;

/// Which bit of the tune are we in?
#[derive(Debug, PartialEq, PartialOrd, Clone, Copy)]
enum TuneSection {
    Header,
    Body,
}

/// Context required to lex an ABC String.
/// Context object is immutable for simpler state and testing.
#[derive(PartialEq, PartialOrd, Clone, Copy)]
pub struct Context<'a> {
    /// The ABC tune content as a vector of potentially multibyte characters.
    /// Stored as a slice of chars so we can peek.
    c: &'a [char],

    // Length of string.
    l: usize,

    // The current index of the string during lexing.
    i: usize,

    tune_section: TuneSection,
}

impl<'a> Context<'a> {
    fn new(c: &'a [char]) -> Context<'a> {

        let l = c.len();

        Context {
            c,
            l,
            i: 0,
            tune_section: TuneSection::Header,
        }
    }

    /// Are there this many characters available?
    fn has(&self, chars: usize) -> bool {
        self.i + chars <= self.l
    }

    /// Move to body section.
    fn in_body(&self) -> Context<'a> {
        Context {
            tune_section: TuneSection::Body,
            ..*self
        }
    }

    /// Skip this many characters.
    fn skip(self, amount: usize) -> Context<'a> {
        let i = self.i + amount;
        Context { i, ..self }
    }

    /// Take the first character, if there is one.
    fn first(&self) -> Option<(Context<'a>, char)> {
        if !self.has(1) {
            None
        } else {
            Some((self.skip(1), self.c[self.i]))
        }
    }

    /// Peek at the first character, if there is one, but don't increment offset.
    fn peek_first(&self) -> Option<(Context<'a>, char)> {
        if !self.has(1) {
            None
        } else {
            Some((*self, self.c[self.i]))
        }
    }

    /// Take the first n characters, if we have them.
    #[test]
    fn take(&self, n: usize) -> Option<(Context<'a>, &'a [char])> {
        if !self.has(n) {
            None
        } else {
            Some((self.skip(n), &self.c[self.i..self.i + n]))
        }
    }

    /// Skip any whitespace at the offset.
    fn skip_whitespace(&self) -> Context<'a> {
        let mut context: Context<'a> = *self;

        // Recursive version didn't TCO.
        loop {
            match context.first() {

                Some((ctx, ' ')) => context = ctx,

                // Non-matching character.
                Some((_, _)) => return context,

                None => return context,
            }
        }
    }

    /// Does the context start with the given string?
    fn starts_with_insensitive_eager(&self, prefix: &'a [char]) -> (Context<'a>, bool) {
        let len = prefix.len();
        if self.i + len > self.l {
            (*self, false)
        } else {
            for i in 0..len {

                // If there's no match return original context's offset.
                if self.c[self.i + i].to_uppercase().next() != prefix[i].to_uppercase().next() {
                    return (*self, false);
                }
            }

            (self.skip(len), true)
        }
    }

    /// Skip an optional prefix, returning true or false for whether or not it matched.
    fn skip_optional_prefix(&self, prefix: &'a [char]) -> Context<'a> {
        if let (ctx, true) = self.starts_with_insensitive_eager(prefix) {
            ctx
        } else {
            *self
        }
    }

    /// The content from the offset onwards.
    #[test]
    fn rest(&self) -> &'a [char] {
        &self.c[self.i..]
    }
}

impl<'a> fmt::Debug for Context<'a> {
    /// Printing the entire buffer makes test debugging unreadable.
    /// Print only the offset and length.
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Ctx: {{ i: {}, length: {} }}", self.i, self.l)
    }
}



/// Read until delmiter character.
/// Return that slice plus the content.
fn read_until<'a>(
    ctx: Context<'a>,
    delimiter: char,
) -> Result<(Context<'a>, &'a [char]), Context<'a>> {

    if let Some(offset) = ctx.c[ctx.i..].iter().position(|c| *c == delimiter) {
        // Skip 1 for the delimiter character.
        Ok((ctx.skip(offset + 1), &ctx.c[ctx.i..ctx.i + offset]))
    } else {
        // If we can't find another delimiter at all anywhere, that must mean it's the end of the
        // ABC input. In which case fast-forward to the end so the error message looks nice.
        let characters_remaining = ctx.l - ctx.i;
        Err(ctx.skip(characters_remaining))
    }
}

/// Read an unsigned integer up to 99999999.
/// Supply a role that the number plays for better error messages.
/// On success return value and context.
/// On failure return context, error offset, and error.
fn read_number<'a>(
    ctx: Context<'a>,
    role: NumberRole,
) -> Result<(Context<'a>, u32), (Context, usize, LexError)> {
    // We're not going to read anything longer than this.
    // Doing so would be unlikely and overflow a u32.
    const MAX_CHARS: usize = 8;
    let mut too_long = false;

    let mut value: u32 = 0;
    let mut length = 0;

    for i in ctx.i..ctx.l {

        // Catch an over-long number before it overflows u32 bits.
        // If it's too long we'll discard the number, but want to leave the context.i at the end
        // of the digits. It's less fiddly to keep resetting the value for the remainder of the bad
        // digit sequence.
        if length >= MAX_CHARS {
            value = 0;
            too_long = true;
        }

        match ctx.c[i] {
            '0' => {
                value *= 10;
                value += 0
            }
            '1' => {
                value *= 10;
                value += 1
            }
            '2' => {
                value *= 10;
                value += 2
            }
            '3' => {
                value *= 10;
                value += 3
            }
            '4' => {
                value *= 10;
                value += 4
            }
            '5' => {
                value *= 10;
                value += 5
            }
            '6' => {
                value *= 10;
                value += 6
            }
            '7' => {
                value *= 10;
                value += 7
            }
            '8' => {
                value *= 10;
                value += 8
            }
            '9' => {
                value *= 10;
                value += 9
            }
            _ => break,
        }

        length += 1;
    }

    if too_long {

        // Set the context to the end of the number, but report error from the start of it.
        let start_of_number = ctx.i;
        return Err((
            ctx.skip(length),
            start_of_number,
            LexError::NumberTooLong(role),
        ));
    } else if length == 0 {
        Err((
            ctx.clone().skip(length),
            ctx.i,
            LexError::ExpectedNumber(role),
        ))
    } else {
        Ok((ctx.skip(length), value))
    }
}

/// Lex a default note length, e.g. "1/9"
fn lex_note_length<'a>(ctx: Context<'a>, delimiter: char) -> LexResult {
    match read_until(ctx, delimiter) {
        Err(ctx) => LexResult::Error(ctx, ctx.i, LexError::PrematureEnd(During::Metre)),

        Ok((whole_line_ctx, _)) => {
            match read_number(ctx, NumberRole::UpperDefaultNoteLength) {
                Err((_, offset, err)) => LexResult::Error(whole_line_ctx, offset, err),
                Ok((ctx, numerator)) => {
                    match ctx.first() {
                        None => LexResult::Error(ctx, ctx.i, LexError::ExpectedSlashInNoteLength),
                        Some((ctx, '/')) => {
                            match read_number(ctx, NumberRole::LowerDefaultNoteLength) {
                                Err((_, offset, err)) => {
                                    LexResult::Error(whole_line_ctx, offset, err)
                                }
                                Ok((ctx, denomenator)) => {
                                    // Skip one character for the delimiter.
                                    LexResult::t(
                                        ctx.skip(1),
                                        T::DefaultNoteLength(
                                            music::FractionalDuration(numerator, denomenator),
                                        ),
                                    )
                                }
                            }
                        }
                        Some((ctx, _)) => {
                            LexResult::Error(ctx, ctx.i, LexError::ExpectedSlashInNoteLength)
                        }
                    }
                }
            }
        }
    }
}


/// Lex a metre declaration, e.g. "2/4" or "C|".
fn lex_metre<'a>(ctx: Context<'a>, delimiter: char) -> LexResult {
    // Read the whole line. This does two things:
    // 1 - Check that the field is actually delimited.
    // 2 - Provide a slice that we can compare to literal values like "C|".
    // However, because the returned context from read_until() places i at the end of whole field,
    // and we still want to parse the values, we won't use this returned context for parsing.
    // We do, however return it from lex_metre(), as it's in the right place to continue lexing,
    // and if there was an error during the line, we return the context in the error at a place
    // we can pick up from.
    match read_until(ctx, delimiter) {
        Err(ctx) => LexResult::Error(ctx, ctx.i, LexError::PrematureEnd(During::Metre)),

        // Although this context is discareded for parsing, it is used to return errors,
        // as it enables the lexer to continue at the next token.
        Ok((whole_line_ctx, content)) => {

            if content == &['C'] {
                LexResult::t(ctx, T::Metre(4, 4))
            } else if content == &['C', '|'] {
                LexResult::t(ctx, T::Metre(2, 4))
            } else {
                // It's a numerical metre.
                match read_number(ctx, NumberRole::UpperTimeSignature) {
                    Err((_, offset, err)) => LexResult::Error(whole_line_ctx, offset, err),
                    Ok((ctx, numerator)) => {
                        match ctx.first() {
                            None => LexResult::Error(ctx, ctx.i, LexError::ExpectedSlashInMetre),
                            Some((ctx, '/')) => {
                                match read_number(ctx, NumberRole::LowerTimeSignature) {
                                    Err((_, offset, err)) => {
                                        LexResult::Error(whole_line_ctx, offset, err)
                                    }
                                    Ok((ctx, denomenator)) => {
                                        // Skip one character for the delimiter.
                                        LexResult::t(ctx.skip(1), T::Metre(numerator, denomenator))
                                    }
                                }
                            }
                            Some((ctx, _)) => {
                                LexResult::Error(ctx, ctx.i, LexError::ExpectedSlashInMetre)
                            }
                        }

                    }

                }

            }
        }
    }
}

/// Lex a key note, e.g. "C", "Bf", "F Flat".
fn read_key_note<'a>(ctx: Context<'a>) -> Option<(Context<'a>, music::PitchClass)> {
    let (ctx, diatonic) = match ctx.first() {
        Some((ctx, 'A')) => (ctx, Some(music::DiatonicPitchClass::A)),
        Some((ctx, 'B')) => (ctx, Some(music::DiatonicPitchClass::B)),
        Some((ctx, 'C')) => (ctx, Some(music::DiatonicPitchClass::C)),
        Some((ctx, 'D')) => (ctx, Some(music::DiatonicPitchClass::D)),
        Some((ctx, 'E')) => (ctx, Some(music::DiatonicPitchClass::E)),
        Some((ctx, 'F')) => (ctx, Some(music::DiatonicPitchClass::F)),
        Some((ctx, 'G')) => (ctx, Some(music::DiatonicPitchClass::G)),

        // If there's no key note, just return the current context unchanged.
        _ => (ctx, None),
    };

    let (ctx, accidental) = match diatonic {
        // If there was no key note, don't try and match an accidental.
        None => (ctx, None),

        // If there was a key note, try and read an accidental.
        // Read longest ones first.
        Some(_) => {
            if let (ctx, true) = ctx.starts_with_insensitive_eager(&['f', 'l', 'a', 't']) {
                (ctx, Some(music::Accidental::Flat))
            } else if let (ctx, true) = ctx.starts_with_insensitive_eager(
                &['s', 'h', 'a', 'r', 'p'],
            )
            {
                (ctx, Some(music::Accidental::Sharp))
            } else if let (ctx, true) = ctx.starts_with_insensitive_eager(
                &['n', 'a', 't', 'u', 'r', 'a', 'l'],
            )
            {
                (ctx, Some(music::Accidental::Natural))
            } else if let (ctx, true) = ctx.starts_with_insensitive_eager(&['f', 'f']) {
                (ctx, Some(music::Accidental::DoubleFlat))
            } else if let (ctx, true) = ctx.starts_with_insensitive_eager(&['s', 's']) {
                (ctx, Some(music::Accidental::DoubleFlat))
            } else if let (ctx, true) = ctx.starts_with_insensitive_eager(&['b', 'b']) {
                (ctx, Some(music::Accidental::DoubleSharp))
            } else if let (ctx, true) = ctx.starts_with_insensitive_eager(&['#', '#']) {
                (ctx, Some(music::Accidental::DoubleSharp))
            } else if let (ctx, true) = ctx.starts_with_insensitive_eager(&['♯', '♯']) {
                (ctx, Some(music::Accidental::DoubleSharp))
            } else if let (ctx, true) = ctx.starts_with_insensitive_eager(&['♭', '♭']) {
                (ctx, Some(music::Accidental::DoubleFlat))
            } else if let (ctx, true) = ctx.starts_with_insensitive_eager(&['f']) {
                (ctx, Some(music::Accidental::Flat))
            } else if let (ctx, true) = ctx.starts_with_insensitive_eager(&['s']) {
                (ctx, Some(music::Accidental::Sharp))
            } else if let (ctx, true) = ctx.starts_with_insensitive_eager(&['b']) {
                (ctx, Some(music::Accidental::Flat))
            } else if let (ctx, true) = ctx.starts_with_insensitive_eager(&['#']) {
                (ctx, Some(music::Accidental::Sharp))
            } else if let (ctx, true) = ctx.starts_with_insensitive_eager(&['♯']) {
                (ctx, Some(music::Accidental::Sharp))
            } else if let (ctx, true) = ctx.starts_with_insensitive_eager(&['♭']) {
                (ctx, Some(music::Accidental::Flat))
            } else if let (ctx, true) = ctx.starts_with_insensitive_eager(&['=']) {
                (ctx, Some(music::Accidental::Natural))
            } else if let (ctx, true) = ctx.starts_with_insensitive_eager(&['♮']) {
                (ctx, Some(music::Accidental::Natural))
            } else {
                (ctx, None)
            }
        }
    };

    match diatonic {
        Some(diatonic) => Some((
            ctx,
            music::PitchClass {
                diatonic_pitch_class: diatonic,
                accidental: accidental,
            },
        )),
        _ => None,
    }
}

/// Read a musical mode.
pub fn read_mode<'a>(ctx: Context<'a>) -> Option<(Context<'a>, music::Mode)> {
    let ctx = ctx.skip_whitespace();

    // Read both long and short forms, and leave ctx at the end if whichever matched.
    // There may be more tokens to follow after this, so it's not enough just to take 'maj',
    // we must search for 'major' first.
    if let (ctx, true) = ctx.starts_with_insensitive_eager(&['m', 'a', 'j', 'o', 'r']) {
        Some((ctx, music::Mode::Major))
    } else if let (ctx, true) = ctx.starts_with_insensitive_eager(&['m', 'i', 'n', 'o', 'r']) {
        Some((ctx, music::Mode::Minor))
    } else if let (ctx, true) = ctx.starts_with_insensitive_eager(&['l', 'y', 'd', 'i', 'a', 'n']) {
        Some((ctx, music::Mode::Lydian))
    } else if let (ctx, true) = ctx.starts_with_insensitive_eager(&['i', 'o', 'n', 'i', 'a', 'n']) {
        Some((ctx, music::Mode::Ionian))
    } else if let (ctx, true) = ctx.starts_with_insensitive_eager(
        &['m', 'i', 'x', 'o', 'l', 'y', 'd', 'i', 'a', 'n'],
    )
    {
        Some((ctx, music::Mode::Mixolydian))
    } else if let (ctx, true) = ctx.starts_with_insensitive_eager(&['d', 'o', 'r', 'i', 'a', 'n']) {
        Some((ctx, music::Mode::Dorian))
    } else if let (ctx, true) = ctx.starts_with_insensitive_eager(
        &['a', 'e', 'o', 'l', 'i', 'a', 'n'],
    )
    {
        Some((ctx, music::Mode::Aeolian))
    } else if let (ctx, true) = ctx.starts_with_insensitive_eager(
        &['p', 'h', 'r', 'y', 'g', 'i', 'a', 'n'],
    )
    {
        Some((ctx, music::Mode::Phrygian))

    } else if let (ctx, true) = ctx.starts_with_insensitive_eager(&['m', 'a', 'j']) {
        Some((ctx, music::Mode::Major))

    } else if let (ctx, true) = ctx.starts_with_insensitive_eager(&['m', 'i', 'n']) {
        Some((ctx, music::Mode::Minor))

    } else if let (ctx, true) = ctx.starts_with_insensitive_eager(&['l', 'o', 'c']) {
        Some((ctx, music::Mode::Locrian))
    } else if let (ctx, true) = ctx.starts_with_insensitive_eager(&['l', 'y', 'd']) {
        Some((ctx, music::Mode::Lydian))
    } else if let (ctx, true) = ctx.starts_with_insensitive_eager(&['i', 'o', 'n']) {
        Some((ctx, music::Mode::Ionian))
    } else if let (ctx, true) = ctx.starts_with_insensitive_eager(&['m', 'i', 'x']) {
        Some((ctx, music::Mode::Mixolydian))
    } else if let (ctx, true) = ctx.starts_with_insensitive_eager(&['d', 'o', 'r']) {
        Some((ctx, music::Mode::Dorian))
    } else if let (ctx, true) = ctx.starts_with_insensitive_eager(&['a', 'e', 'o']) {
        Some((ctx, music::Mode::Aeolian))
    } else if let (ctx, true) = ctx.starts_with_insensitive_eager(&['p', 'h', 'r']) {
        Some((ctx, music::Mode::Phrygian))
    } else if let (ctx, true) = ctx.starts_with_insensitive_eager(&['l', 'o', 'c']) {
        Some((ctx, music::Mode::Locrian))
    } else {
        None
    }
}

/// Read a fractional duration. This can be notated as zero characters.
fn read_fractional_duration<'a>(ctx: Context<'a>) -> (Context, music::FractionalDuration) {
    // Get a number, if present.
    let (ctx, numerator) = match read_number(ctx, NumberRole::NoteDurationNumerator) {
        Ok((ctx, val)) => (ctx, Some(val)),
        Err((ctx, _, _)) => (ctx, None),
    };

    // Read a slash, if there is one.
    let (ctx, denomenator, has_slash) =
        if let (ctx, true) = ctx.starts_with_insensitive_eager(&['/']) {
            // If there is a slash then read the denomenator (which can be empty).
            match read_number(ctx, NumberRole::NoteDurationNumerator) {
                Ok((ctx, val)) => (ctx, Some(val), true),

                // No number after the slash, default to 1.
                Err((ctx, _, _)) => (ctx, None, true),
            }


        } else {
            // No slash, so don't expect to read a denomenator.
            (ctx, None, false)
        };

    // We need to handle the shorthand, as missing numbers mean different things in different
    // contexts.
    let (numerator, denomenator) = match (numerator, denomenator, has_slash) {
        // e..g "/". Special case, which means "1/2".
        (None, None, true) => (1, 2),

        // e.g. "". Default note length.
        (None, None, false) => (1, 1),

        // e.g. "/2". divide by this amount.
        (None, Some(d), true) => (1, d),

        // e.g. "1/".
        (Some(n), None, true) => (n, 1),

        // e.g. "1/2".
        (Some(n), Some(d), true) => (n, d),

        // e.g. "2"
        (Some(n), None, false) => (n, 1),

        // This should never happen. If it does, use the standard note length.
        _ => (1, 1),
    };

    (ctx, music::FractionalDuration(numerator, denomenator))
}


fn lex_key_signature<'a>(ctx: Context<'a>, delimiter: char) -> LexResult {
    match read_until(ctx, delimiter) {
        Err(ctx) => LexResult::Error(ctx, ctx.i, LexError::PrematureEnd(During::KeySignature)),

        // Although this context is discareded for parsing, it is used to return errors,
        // as it enables the lexer to continue at the next token.
        Ok((whole_line_ctx, _)) => {
            if let Some((ctx, key_note)) = read_key_note(ctx) {

                // TODO: Assuming empty means 'major'. Is this correct for at the lexer?
                // Or maybe the AST-level representation should handle the behaviour.
                let (_, mode) = read_mode(ctx).unwrap_or((ctx, music::Mode::Major));

                // TODO extras like specific accidentals?

                // Skip to end of delimited sequence (line or bracket).
                LexResult::t(whole_line_ctx, T::KeySignature(key_note, mode))
            } else {
                // TODO: There may be an alternative to a key-note. May need to amend this when
                // fuzzing with real-world inputs.
                LexResult::Error(ctx, ctx.i, LexError::UnrecognisedKeyNote)
            }
        }
    }
}


/// Read an n-time-repeat, e.g. "[2" or "2" immediately following a barline.
fn read_n_time<'a>(ctx: Context<'a>) -> (Context<'a>, Option<u32>) {

    let ctx = ctx.skip_optional_prefix(&['[']);


    match read_number(ctx, NumberRole::NTimeBar) {
        Ok((ctx, number)) => (ctx, Some(number)),
        _ => (ctx, None),
    }
}

/// Lex a barline, when it is expected.
/// TODO all tests for this!
fn lex_barline<'a>(ctx: Context<'a>) -> LexResult {

    if let (ctx, true) = ctx.starts_with_insensitive_eager(&[':', '|', ':']) {
        LexResult::t(
            ctx,
            T::Barline(music::Barline {
                repeat_before: true,
                single: true,
                repeat_after: true,
                n_time: None,
            }),
        )
    } else if let (ctx, true) = ctx.starts_with_insensitive_eager(&[':', '|', '|', ':']) {
        LexResult::t(
            ctx,
            T::Barline(music::Barline {
                repeat_before: true,
                single: false,
                repeat_after: true,
                n_time: None,
            }),
        )
    } else if let (ctx, true) = ctx.starts_with_insensitive_eager(&[':', '|']) {

        let (ctx, n_time) = read_n_time(ctx);

        LexResult::t(
            ctx,
            T::Barline(music::Barline {
                repeat_before: true,
                single: true,
                repeat_after: false,
                n_time: n_time,
            }),
        )
    } else if let (ctx, true) = ctx.starts_with_insensitive_eager(&['|', '|', ':']) {
        LexResult::t(
            ctx,
            T::Barline(music::Barline {
                repeat_before: false,
                single: true,
                repeat_after: true,
                n_time: None,
            }),
        )
    } else if let (ctx, true) = ctx.starts_with_insensitive_eager(&['|', ':']) {
        LexResult::t(
            ctx,
            T::Barline(music::Barline {
                repeat_before: false,
                single: true,
                repeat_after: true,
                n_time: None,
            }),
        )
    } else if let (ctx, true) = ctx.starts_with_insensitive_eager(&[':', '|', '|']) {
        let (ctx, n_time) = read_n_time(ctx);

        LexResult::t(
            ctx,
            T::Barline(music::Barline {
                repeat_before: true,
                single: false,
                repeat_after: false,
                n_time: n_time,
            }),
        )

    } else if let (ctx, true) = ctx.starts_with_insensitive_eager(&['|', '|']) {
        let (ctx, n_time) = read_n_time(ctx);
        LexResult::t(
            ctx,
            T::Barline(music::Barline {
                repeat_before: false,
                single: false,
                repeat_after: false,
                n_time: n_time,
            }),
        )
    } else if let (ctx, true) = ctx.starts_with_insensitive_eager(&['|', ']']) {
        let (ctx, n_time) = read_n_time(ctx);
        LexResult::t(
            ctx,
            T::Barline(music::Barline {
                repeat_before: false,
                single: false,
                repeat_after: false,
                n_time: n_time,
            }),
        )
    } else if let (ctx, true) = ctx.starts_with_insensitive_eager(&['|', ':']) {
        LexResult::t(
            ctx,
            T::Barline(music::Barline {
                repeat_before: false,
                single: false,
                repeat_after: false,
                n_time: None,
            }),
        )
    } else if let (ctx, true) = ctx.starts_with_insensitive_eager(&[':', ':']) {
        LexResult::t(
            ctx,
            T::Barline(music::Barline {
                repeat_before: true,
                single: true,
                repeat_after: true,
                n_time: None,
            }),
        )
    } else if let (ctx, true) = ctx.starts_with_insensitive_eager(&[':', '|', ':']) {
        LexResult::t(
            ctx,
            T::Barline(music::Barline {
                repeat_before: true,
                single: false,
                repeat_after: true,
                n_time: None,
            }),
        )
    } else if let (ctx, true) = ctx.starts_with_insensitive_eager(&['|']) {
        let (ctx, n_time) = read_n_time(ctx);
        LexResult::t(
            ctx,
            T::Barline(music::Barline {
                repeat_before: false,
                single: true,
                repeat_after: false,
                n_time: n_time,
            }),
        )
    } else {
        LexResult::Error(ctx, ctx.i, LexError::UnrecognisedBarline)
    }
}

fn lex_note<'a>(ctx: Context<'a>) -> LexResult {
    // Optional accidental.

    let (ctx, accidental) = if let (ctx, true) = ctx.starts_with_insensitive_eager(&['^', '^']) {
        (ctx, Some(music::Accidental::DoubleSharp))
    } else if let (ctx, true) = ctx.starts_with_insensitive_eager(&['^', '^']) {
        (ctx, Some(music::Accidental::DoubleSharp))
    } else if let (ctx, true) = ctx.starts_with_insensitive_eager(&['_', '_']) {
        (ctx, Some(music::Accidental::DoubleFlat))
    } else if let (ctx, true) = ctx.starts_with_insensitive_eager(&['^']) {
        (ctx, Some(music::Accidental::Sharp))
    } else if let (ctx, true) = ctx.starts_with_insensitive_eager(&['_']) {
        (ctx, Some(music::Accidental::Flat))
    } else if let (ctx, true) = ctx.starts_with_insensitive_eager(&['=']) {
        (ctx, Some(music::Accidental::Natural))
    } else {
        (ctx, None)
    };

    let (ctx, diatonic, octave) = match ctx.first() {
        Some((ctx, 'A')) => (ctx, Some(music::DiatonicPitchClass::A), 0),
        Some((ctx, 'B')) => (ctx, Some(music::DiatonicPitchClass::B), 0),
        Some((ctx, 'C')) => (ctx, Some(music::DiatonicPitchClass::C), 0),
        Some((ctx, 'D')) => (ctx, Some(music::DiatonicPitchClass::D), 0),
        Some((ctx, 'E')) => (ctx, Some(music::DiatonicPitchClass::E), 0),
        Some((ctx, 'F')) => (ctx, Some(music::DiatonicPitchClass::F), 0),
        Some((ctx, 'G')) => (ctx, Some(music::DiatonicPitchClass::G), 0),
        Some((ctx, 'a')) => (ctx, Some(music::DiatonicPitchClass::A), 1),
        Some((ctx, 'b')) => (ctx, Some(music::DiatonicPitchClass::B), 1),
        Some((ctx, 'c')) => (ctx, Some(music::DiatonicPitchClass::C), 1),
        Some((ctx, 'd')) => (ctx, Some(music::DiatonicPitchClass::D), 1),
        Some((ctx, 'e')) => (ctx, Some(music::DiatonicPitchClass::E), 1),
        Some((ctx, 'f')) => (ctx, Some(music::DiatonicPitchClass::F), 1),
        Some((ctx, 'g')) => (ctx, Some(music::DiatonicPitchClass::G), 1),

        _ => (ctx, None, 0),
    };

    // Optional octave modifier.
    let (ctx, octave) = match ctx.peek_first() {
        Some((ctx, ',')) => (ctx, octave - 1),
        Some((ctx, '\'')) => (ctx, octave + 1),
        _ => (ctx, 0),
    };

    // Duration has a few different representations, including zero characters.
    let (ctx, duration) = read_fractional_duration(ctx);

    if let Some(diatonic) = diatonic {
        LexResult::t(
            ctx,
            T::Note(music::Note(
                music::Pitch {
                    pitch_class: music::PitchClass {
                        diatonic_pitch_class: diatonic,
                        accidental: accidental,
                    },
                    octave: octave,
                },
                duration,
            )),
        )
    } else {
        LexResult::Error(ctx, ctx.i, LexError::UnrecognisedNote)
    }
}



// The activity we were undertaking at the time when something happened.
#[derive(Debug, PartialEq, PartialOrd, Clone)]
pub enum During {
    Metre,

    // General purpose header section.
    Header,

    KeySignature,

    DefaultNoteLenth,
}

#[derive(Debug, PartialEq, PartialOrd, Clone)]
pub enum NumberRole {
    UpperTimeSignature,
    LowerTimeSignature,
    NoteDurationNumerator,
    NoteDurationDenomenator,
    UpperDefaultNoteLength,
    LowerDefaultNoteLength,
    NTimeBar,
}

/// Types of errors. These should be as specific as possible to give the best help.
/// Avoiding generic 'expected char' type values.
#[derive(Debug, PartialEq, PartialOrd, Clone)]
pub enum LexError {
    /// We expected to find a delimiter at some point after the current position but couldn't.
    ExpectedDelimiter(char),

    /// We expected a field type (e.g. "T") but didn't get one.
    ExpectedFieldType(char),

    /// We expected to find a colon character.
    ExpectedColon,

    /// We expected to find a number here.
    ExpectedNumber(NumberRole),

    /// During a metre declaration, expected to get slash.
    ExpectedSlashInMetre,

    /// Number is too long.
    NumberTooLong(NumberRole),

    /// Premature end of file. We expected something else here.
    PrematureEnd(During),

    /// In the tune header, we found a start of line that we couldn't recognise.
    UnexpectedHeaderLine,

    /// In the tune body, where we expect the start of a token, we got a character we didn't expect.
    UnexpectedBodyChar(char),

    /// Feature not implemented yet.
    /// Should have no tests for this.
    /// Marker value for tracking down callsite.
    /// TODO remove this when feature complete.
    UnimplementedError(u32),

    // ExpectedKeySignature,
    UnrecognisedKeyNote,

    UnrecognisedBarline,

    UnrecognisedNote,

    ExpectedSlashInNoteLength,
}

/// Indent and print a line to a string buffer.
/// This is used for all subsequent lines in an error message (the first is already indented).
fn indent_and_append_line(indent: usize, buf: &mut String, string: &String) {
    for _ in 0..indent {
        buf.push(' ');
    }
    buf.push_str(string);
    buf.push('\n')
}

/// Indent and print a sequence of lines.
fn indent_and_append_lines(indent: usize, buf: &mut String, lines: &[&String]) {
    for line in lines.iter() {
        indent_and_append_line(indent, buf, line);
    }
}

impl LexError {
    /// Format the error to the string buffer.
    /// If more than one line is used, indent by this much.
    /// Don't append a newline.
    pub fn format(&self, indent: usize, buf: &mut String) {
        match self {
            &LexError::ExpectedDelimiter(chr) => {
                // Printing \n is confusing.
                if chr == '\n' {
                    buf.push_str("I expected to find a new-line here.");
                } else {
                    buf.push_str("I expected to find the character '");
                    buf.push(chr);
                    buf.push_str("' here.");
                }
            }
            &LexError::ExpectedColon => {
                buf.push_str("I expected to see a colon here.");
            }
            &LexError::ExpectedFieldType(chr) => {
                buf.push_str("I found a header of '");
                buf.push(chr);
                buf.push_str("' but I don't understand it.\n");

                // TODO ugly
                indent_and_append_lines(
                    indent,
                    buf,
                    &[
                        &"Recognised headers:".to_string(),
                        &"A: Geographical Area".to_string(),
                        &"B: Book".to_string(),
                        &"C: Composer".to_string(),
                        &"D: Discography".to_string(),
                        &"F: File Name".to_string(),
                        &"G: Group".to_string(),
                        &"H: History".to_string(),
                        &"I: Information".to_string(),
                        &"K: Key".to_string(),
                        &"L: Default note length".to_string(),
                        &"M: Meter".to_string(),
                        &"N: Notes".to_string(),
                        &"O: Geographical Origin".to_string(),
                        &"P: Parts".to_string(),
                        &"Q: Tempo".to_string(),
                        &"R: Rhythm".to_string(),
                        &"S: Source".to_string(),
                        &"T: Title".to_string(),
                        &"W: Words".to_string(),
                        &"X: Tune number".to_string(),
                        &"Z: Transcription note".to_string(),
                    ],
                );

            }
            &LexError::ExpectedNumber(ref number_role) => {
                buf.push_str("I expected to find a number here.\n");
                match number_role {
                    &NumberRole::UpperTimeSignature => {
                        indent_and_append_line(
                            indent,
                            buf,
                            &"I expected the first / upper part of a time signature.".to_string(),
                        )
                    }
                    &NumberRole::LowerTimeSignature => {
                        indent_and_append_line(
                            indent,
                            buf,
                            &"I expected the second / lower part of a time signature.".to_string(),
                        )
                    }

                    // NoteDurationNumerator and NoteDurationDenomenator shouldn't ever actually
                    // occur as they are read in an optional context, but if they do, be polite.
                    &NumberRole::NoteDurationNumerator => {
                        indent_and_append_line(
                            indent,
                            buf,
                            &"I expected to find a number for a note length.".to_string(),
                        )
                    }

                    &NumberRole::NoteDurationDenomenator => {
                        indent_and_append_line(
                            indent,
                            buf,
                            &"I expected to find a number for a note length.".to_string(),
                        )
                    }
                    &NumberRole::UpperDefaultNoteLength => {
                        indent_and_append_line(
                            indent,
                            buf,
                            &"I expected to find the first / upper part of a default note length."
                                .to_string(),
                        )
                    }
                    &NumberRole::LowerDefaultNoteLength => {
                        indent_and_append_line(
                            indent,
                            buf,
                            &"I expected to find the second / lower part of a default note length."
                                .to_string(),
                        )
                    }
                    &NumberRole::NTimeBar => {
                        indent_and_append_line(
                            indent,
                            buf,
                            &"I expected to find a n-time repeat bar.".to_string(),
                        )
                    }

                }
            }
            &LexError::ExpectedSlashInMetre => {
                buf.push_str("I expected to find a slash for the time signature.");
            }
            &LexError::NumberTooLong(_) => {
                buf.push_str("This number is longer than I expected.");
            }
            &LexError::PrematureEnd(ref during) => {
                buf.push_str("I've got to the end of the ABC input before I'm ready.\n");
                match during {
                    &During::Metre => {
                        indent_and_append_line(
                            indent,
                            buf,
                            &"I was in the middle of reading a time signature".to_string(),
                        )
                    }
                    &During::Header => {
                        indent_and_append_line(
                            indent,
                            buf,
                            &"I was in the middle of reading a header field.".to_string(),
                        )
                    }
                    &During::KeySignature => {
                        indent_and_append_line(
                            indent,
                            buf,
                            &"I was in the middle of reading a key signature.".to_string(),
                        )
                    }
                    &During::DefaultNoteLenth => {
                        indent_and_append_line(
                            indent,
                            buf,
                            &"I was in the middle of reading a default note length.".to_string(),
                        )
                    }
                }
            }
            &LexError::UnexpectedBodyChar(chr) => {
                buf.push_str("I didn't expect to find the character '");
                buf.push(chr);
                buf.push_str("' here.");
            }
            &LexError::UnexpectedHeaderLine => {
                buf.push_str("I expected to find a header, but found something else.");
            }
            &LexError::UnimplementedError(ident) => {
                buf.push_str(
                    "I'm confused, sorry. Please email joe@afandian.com with your ABC \
                              and quote number '",
                );
                buf.push_str(&ident.to_string());
                buf.push_str("' and I'll see if I can fix it.");
            }
            &LexError::UnrecognisedKeyNote => {
                buf.push_str(
                    "I expected to find a tonic for a key signature, but didn't understand this.",
                );
            }
            &LexError::ExpectedSlashInNoteLength => {
                buf.push_str(
                    "I expected to find a slash character in a default note length.",
                );
            }
            &LexError::UnrecognisedBarline => {
                buf.push_str("I couldn't understand this bar line.");
            }
            &LexError::UnrecognisedNote => {
                buf.push_str("I didn't understand how to read this note.");
            }

        }
    }
}

/// A glorified Option type that allows encoding errors.
#[derive(Debug, PartialEq, PartialOrd, Clone)]
pub enum LexResult<'a> {
    /// Token. Shortened as it's used a lot.
    T(Context<'a>, Vec<T>),
    /// Error contains a context and an offset of where the error occurred.
    /// The context's offset is used to resume, and should point to the end of the troublesome bit.
    /// The error's offset indidates where the error happened, i.e. the start of the bother.
    Error(Context<'a>, usize, LexError),

    /// End of the file was reached.
    /// Not a token.
    Terminal,
}

impl<'a> LexResult<'a> {
    /// Build a lex result with a single Token.
    fn t(ctx: Context<'a>, t: T) -> LexResult<'a> {
        LexResult::T(ctx, vec![t])
    }

    /// Build a lex result with a number of Tokens.
    fn ts(ctx: Context<'a>, ts: Vec<T>) -> LexResult<'a> {
        LexResult::T(ctx, ts)
    }
}

/// ABC Token.
/// Shortened as it's used a lot.
#[derive(Debug, PartialEq, PartialOrd, Clone)]
pub enum T {
    Newline,
    BeamBreak,

    // Text header fields.
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

    // More interesting header fields.
    Metre(u32, u32),
    KeySignature(music::PitchClass, music::Mode),
    DefaultNoteLength(music::FractionalDuration),

    Barline(music::Barline),

    Note(music::Note),
}

/// Try to read a single T and return a new context.
/// Note that there's a lot of aliasing of ctx in nested matches.
fn read(ctx: Context) -> LexResult {
    match ctx.peek_first() {
        None => LexResult::Terminal,
        Some((ctx, first_char)) => {
            match ctx.tune_section {
                TuneSection::Header => {

                    // We know that in this branch we always want to match on the first char, so can
                    // safely skip now.
                    let ctx = ctx.skip(1);

                    match first_char {
                        // Text headers.
                        'A' | 'B' | 'C' | 'D' | 'F' | 'G' | 'H' | 'I' | 'N' | 'O' | 'R' | 'S' |
                        'T' | 'W' | 'X' | 'Z' => {
                            match ctx.first() {
                                Some((ctx, ':')) => {
                                    match read_until(ctx, '\n') {
                                        Ok((ctx, chars)) => {

                                            let value: String = chars.iter().collect();

                                            // Strip whitespace including leading space and trailing
                                            // newline
                                            let value = value.trim().to_string();

                                            match first_char {
                                                'A' => LexResult::t(ctx, T::Area(value)),
                                                'B' => LexResult::t(ctx, T::Book(value)),
                                                'C' => LexResult::t(ctx, T::Composer(value)),
                                                'D' => LexResult::t(ctx, T::Discography(value)),
                                                'F' => LexResult::t(ctx, T::Filename(value)),
                                                'G' => LexResult::t(ctx, T::Group(value)),
                                                'H' => LexResult::t(ctx, T::History(value)),
                                                'I' => LexResult::t(ctx, T::Information(value)),
                                                'N' => LexResult::t(ctx, T::Notes(value)),
                                                'O' => LexResult::t(ctx, T::Origin(value)),
                                                'S' => LexResult::t(ctx, T::Source(value)),
                                                'T' => LexResult::t(ctx, T::Title(value)),
                                                'W' => LexResult::t(ctx, T::Words(value)),
                                                'X' => LexResult::t(ctx, T::X(value)),
                                                'Z' => LexResult::t(ctx, T::Transcription(value)),

                                                // This can only happen if the above cases get out
                                                // of sync.
                                                _ => {
                                                    LexResult::Error(
                                                        ctx,
                                                        ctx.i,
                                                        LexError::ExpectedFieldType(first_char),
                                                    )
                                                }
                                            }
                                        }
                                        Err(ctx) => {
                                            LexResult::Error(
                                                ctx,
                                                ctx.i,
                                                LexError::ExpectedDelimiter('\n'),
                                            )
                                        }
                                    }
                                }

                                // Not a colon.
                                Some((ctx, _)) => {
                                    LexResult::Error(ctx, ctx.i, LexError::ExpectedColon)
                                }

                                // Unexpected end of file.
                                None => {
                                    LexResult::Error(
                                        ctx,
                                        ctx.i,
                                        LexError::PrematureEnd(During::Header),
                                    )
                                }
                            }
                        }

                        // Non-text headers.
                        // Grouped for handling code.
                        'K' | 'L' | 'M' | 'P' | 'Q' => {
                            match ctx.first() {
                                Some((ctx, ':')) => {

                                    // Skip leading whitespace within the header.
                                    let ctx = ctx.skip_whitespace();
                                    match first_char {

                                        // Key signature.
                                        'K' => {

                                            // K signals a switch to the body section, even if it
                                            // failed to parse.
                                            let ctx = ctx.in_body();

                                            return lex_key_signature(ctx, '\n');
                                        }

                                        // Default note length.
                                        'L' => return lex_note_length(ctx, '\n'),

                                        // Metre.
                                        'M' => return lex_metre(ctx, '\n'),

                                        // Parts.
                                        'P' => {
                                            return LexResult::Error(
                                                ctx,
                                                ctx.i,
                                                LexError::UnimplementedError(3),
                                            )
                                        }

                                        // Tempo
                                        'Q' => {
                                            return LexResult::Error(
                                                ctx,
                                                ctx.i,
                                                LexError::UnimplementedError(4),
                                            )
                                        }

                                        // This can only happen if the above cases get out of sync.
                                        _ => {
                                            return LexResult::Error(
                                                ctx,
                                                ctx.i,
                                                LexError::ExpectedFieldType(first_char),
                                            )
                                        }
                                    }
                                }

                                // Not a colon.
                                Some((ctx, _)) => {
                                    LexResult::Error(ctx, ctx.i, LexError::ExpectedColon)
                                }

                                // Unexpected end of file.
                                None => {
                                    LexResult::Error(
                                        ctx,
                                        ctx.i,
                                        LexError::PrematureEnd(During::Header),
                                    )
                                }
                            }
                        }

                        // Anything else in the header is unrecognised.
                        _ => LexResult::Error(ctx, ctx.i, LexError::UnexpectedHeaderLine),
                    }
                }

                TuneSection::Body => {
                    match first_char {
                        ' ' => LexResult::t(ctx.skip(1), T::BeamBreak),
                        '\n' => LexResult::t(ctx.skip(1), T::Newline),

                        '|' | ':' => lex_barline(ctx),

                        'a' | 'b' | 'c' | 'd' | 'e' | 'f' | 'g' | 'A' | 'B' | 'C' | 'D' | 'E' |
                        'F' | 'G' | '^' | '_' | '=' => lex_note(ctx),

                        // TODO all tune body entities.
                        _ => LexResult::Error(ctx, ctx.i, LexError::UnexpectedBodyChar(first_char)),
                    }
                }
            }
        }
    }
}

/// A stateful lexer for an ABC string.
/// Implements Iterator.
pub struct Lexer<'a> {
    context: Context<'a>,

    // Was the last result an error?
    // Used to attempt to skip over bad input.
    error: Option<LexError>,
}

impl<'a> Lexer<'a> {
    pub fn new(content: &'a [char]) -> Lexer<'a> {
        let context = Context::new(&content);

        Lexer {
            context,
            error: None,
        }
    }

    // Skip into the body. For testing only.
    #[cfg(test)]
    fn in_body(mut self) -> Lexer<'a> {
        self.context = self.context.in_body();
        self
    }

    /// Collect all tokens into vector, ignoring errors.
    pub fn collect_tokens(self) -> Vec<T> {
        self.filter_map(|x| match x {
            LexResult::T(_, tokens) => Some(tokens),
            LexResult::Error(_, _, _) => None,
            LexResult::Terminal => None,
        }).flat_map(|x| x)
            .collect::<Vec<T>>()
    }

    pub fn collect_errors(self) -> Vec<(Context<'a>, usize, LexError)> {
        self.filter_map(|x| match x {
            LexResult::Error(ctx, offset, err) => Some((ctx, offset, err)),
            _ => None,
        }).collect::<Vec<(Context<'a>, usize, LexError)>>()
    }
}

impl<'a> Iterator for Lexer<'a> {
    type Item = LexResult<'a>;

    fn next(&mut self) -> Option<LexResult<'a>> {
        // If we got an error last time we may want to skip over the input to try and resume.
        let skip_amount = match self.error {

            // The errors returned by Metre recover by themselves, so no need to skip.
            Some(LexError::NumberTooLong(NumberRole::UpperTimeSignature)) |
            Some(LexError::NumberTooLong(NumberRole::LowerTimeSignature)) |
            Some(LexError::ExpectedNumber(NumberRole::LowerTimeSignature)) |
            Some(LexError::ExpectedNumber(NumberRole::UpperTimeSignature)) => 0,

            // If there was an error that we haven't deliberately discounted,
            // increment by one to try and recover.
            Some(_) => 1,

            // No error, no increment.
            _ => 0,
        };

        self.context = self.context.clone().skip(skip_amount);
        self.error = None;

        // Take a temporary clone of self.context so it can be consumed.
        // TODO could read() work with a ref?
        let result = read(self.context.clone());

        match result {
            // Stop iteration when we reach the terminal.
            LexResult::Terminal => None,

            // If it's an error, return it and set the flag.
            LexResult::Error(context, offset, error) => {
                self.context = context.clone();
                self.error = Some(error.clone());
                Some(LexResult::Error(context, offset, error))
            }

            // Otherwise it's a token.
            LexResult::T(context, tokens) => {
                self.context = context.clone();
                Some(LexResult::T(context, tokens))
            }
        }
    }
}

/// Parse an ABC input, return nicely formatted error message and number of lex errors.
pub fn format_error_message<'a>(
    input: &[char],
    all_errors: Vec<(Context<'a>, usize, LexError)>,
) -> (usize, u32, String) {
    const ABC_PREFIX: &str = "   ";
    const ERR_PREFIX: &str = "!  ";

    // let all_errors = Lexer::new(&input).collect_errors();

    let length = input.len();

    // String buffer of the error message.
    // Assume that we'll need around double the ABC input.
    // TODO Instrument this on the corpus of ABC tunes.
    let mut buf = String::with_capacity(input.len() * 2);

    // The number of messages that we didn't show.
    // This happens if there's more than one error at a particular index.
    // The lexer shouldn't produce this, but if it does, we want to catch and explain it.
    let mut num_unshown = 0;

    // Start and end index of the most recent line.
    let mut start_of_line;
    let mut end_of_line = 0;

    // For each line we save the errors that occurred at each index.
    let mut error_index: Vec<Option<LexError>> = Vec::with_capacity(100);

    // Indent the first line.
    buf.push_str(ABC_PREFIX);
    let mut first = true;
    for i in 0..input.len() {

        // Deal both with empty strings and non-empty ones.
        let last_char = i + 1 >= length;

        let c = input[i];

        buf.push(c);

        // If it's a newline.
        // If we get a \r\n\ sequence, the \n will still be the last character.
        if c == '\n' || last_char {

            // Start of line is the end of the previous one, plus its newline.
            // Bit of a hack for the starting line, which isn't preceeded by a newline.
            start_of_line = if first {
                first = false;
                0
            } else {
                end_of_line + 1
            };
            end_of_line = i;

            // If it's the last character and we don't get the benefit of a newline, it'll mess up
            // any error formatting that should be shown under the line. So insert one.
            // TODO can we accomplish the same thing just by appending a newline to the input?
            if last_char && c != '\n' {
                buf.push('\n');
                end_of_line += 1;
            }

            let length = (end_of_line - start_of_line) + 1;

            // This doesn't allocate.
            error_index.resize(0, None);
            error_index.resize(length, None);

            // Build the index of errors per character on this line.
            for &(_, offset, ref error) in all_errors.iter() {
                if offset >= start_of_line && offset <= end_of_line {
                    let index_i = offset - start_of_line;

                    // If there  was more than one error at this index, take only the first.
                    // This is because it would be visually confusing and not much help to show
                    // two messages coming from the same character. Also, the first one is
                    // probably more useful, as subsequent ones would be caused by the lexer
                    // being in a weird state.
                    match error_index[index_i] {
                        // Copy the error. It's only a small value type and this is practical.
                        // than copy a reference and get lifetimes involved.
                        None => error_index[index_i] = Some(error.clone()),
                        Some(_) => num_unshown += 1,
                    }

                }
            }

            // We're going to print a pyramid of error messages to accommodate multiple errors per
            // line.  Outer loop decides which error we're going to print, inner loop does the
            // indentation.
            let mut first_line_of_error = true;
            for error_line in error_index.iter().rev() {
                let mut indent = 0;

                match *error_line {
                    None => (),
                    Some(ref error) => {
                        buf.push_str(ERR_PREFIX);
                        indent += ERR_PREFIX.len();

                        for error_char in error_index.iter() {

                            match *error_char {
                                None => {
                                    buf.push(' ');
                                    indent += 1
                                }
                                Some(_) => {
                                    buf.push(if first_line_of_error {
                                        '▲'
                                    } else {
                                        if error_line == error_char {
                                            '┗'
                                        } else {
                                            '┃'
                                        }

                                    });

                                    if error_char == error_line {
                                        buf.push_str(&" ");
                                        indent += 2;
                                        error.format(indent, &mut buf);

                                        // If we reached the target error, don't keep scanning line.
                                        break;
                                    };

                                    indent += 1;
                                }
                            }
                        }
                        buf.push('\n');
                        first_line_of_error = false;
                    }

                }

            }


            // Indent the next line.
            buf.push_str(ABC_PREFIX);
        }

    }

    (all_errors.len(), num_unshown, buf)
}


/// Parse an ABC input, return nicely formatted error message and number of lex errors.
pub fn format_error_message_from_abc(input: &[char]) -> (usize, u32, String) {
    let all_errors = Lexer::new(&input).collect_errors();
    format_error_message(&input, all_errors)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn string_to_vec(input: String) -> Vec<char> {
        input.chars().collect::<Vec<char>>()
    }

    const EMPTY: &str = "";

    const BUTTERFLY: &str = "X:24
T:Butterfly, The
M:9/8
L:1/8
C:John Potts
E:15
Z:Boston
R:slip jig
K:EDor
B2EG2EF3|B2EG2E FED|B2EG2EF3|B2dd2B AFD:|
B2=ce2fg3|B2d g2e dBA|B2=ce2fg2a|b2ag2e dBA:|
B2BB2AG2A|B3 BAB dBA|~B3 B2AG2A|B2dg2e dBA:|";


    #[test]
    fn context_has() {
        //
        // Empty
        //
        let empty = string_to_vec(String::from(EMPTY));
        let some = string_to_vec(String::from(BUTTERFLY));
        let empty_context = Context::new(&empty);
        let some_context = Context::new(&some);

        assert_eq!(
            empty_context.has(0),
            true,
            "Empty string has at least 0 characters"
        );

        assert_eq!(
            empty_context.take(0),
            Some((empty_context, &(vec![])[..])),
            "Empty input take zero returns empty, context unchanged."
        );

        assert_eq!(
            empty_context.has(1),
            false,
            "Empty string doesn't have one characters."
        );

        assert_eq!(
            empty_context.has(20),
            false,
            "Empty string doesn't lots of characters."
        );

        assert_eq!(empty_context.take(5), None, "Empty input can't take any.");


        //
        // Non-empty
        //

        assert_eq!(
            some_context.has(0),
            true,
            "Empty string has at least 0 characters"
        );

        assert_eq!(
            some_context.take(0),
            Some((some_context, &(vec![])[..])),
            "Empty input take zero returns subsequence, context reflects this."
        );

        assert_eq!(
            some_context.has(1),
            true,
            "Empty string has one characters."
        );

        assert_eq!(
            some_context.has(20),
            true,
            "Empty string has lots of characters."
        );

        assert_eq!(
            some_context.take(5),
            Some((
                some_context.skip(5),
                &(vec!['X', ':', '2', '4', '\n'])[..],
            )),
            "Empty input can't take any."
        );
    }

    #[test]
    fn context_skip_whitespace() {
        let empty = string_to_vec("".to_string());
        let some = string_to_vec("   hello".to_string());
        let none = string_to_vec("hello".to_string());

        assert_eq!(
            Context::new(&empty).skip_whitespace(),
            Context::new(&empty),
            "skip_whitespace() on empty string makes no change"
        );

        assert_eq!(
            Context::new(&some).skip_whitespace(),
            Context::new(&some).skip(3),
            "skip_whitespace() skips to first non-whitespace character"
        );

        assert_eq!(
            Context::new(&none).skip_whitespace(),
            Context::new(&none).skip(0),
            "skip_whitespace() no change when no whitespace"
        );
    }


    #[test]
    fn lexer_can_skip_err() {
        // Input has one good field, one with an error, then another good one.
        let input = &(string_to_vec("T:Title\nM:6/\nC:Composer\n".to_string()));

        // The iterator's result should include all errors and context.
        let all_results = Lexer::new(input).collect::<Vec<LexResult>>();

        // Check that we returned token, error, token.
        match all_results[0] {
            LexResult::T(_, ref tokens) => assert_eq!(tokens, &[T::Title("Title".to_string())]),
            _ => assert!(false),
        }

        match all_results[1] {
            LexResult::Error(_, _, LexError::ExpectedNumber(NumberRole::LowerTimeSignature)) => {
                assert!(true)
            }
            _ => assert!(false),
        }

        match all_results[2] {
            LexResult::T(_, ref tokens) => {
                assert_eq!(tokens, &[T::Composer("Composer".to_string())])
            }
            _ => assert!(false),
        }

        // The collect_tokens() allows collection of tokens ignoring the errors.
        assert_eq!(
            Lexer::new(input).collect_tokens(),
            vec![
                T::Title("Title".to_string()),
                T::Composer("Composer".to_string()),
            ]
        );
    }

    // Test for every header to make sure everything hangs together.
    #[test]
    fn read_headers_test() {
        // Some have leading whitespace, which should be ignored.
        let input = &(string_to_vec(
            "A:AREA
B:BOOK
C:COMPOSER
D:DISCOGRAPHY
F:FILENAME
G: GROUP
H:HISTORY
I:INFO
N:NOTES
O:ORIGIN
S:SOURCE
T:TITLE
W:WORDS
X:100
Z:TRANSCRIPTION
M:2/4
M:        5/8
L:1/8
K:    GFmaj
"
                .to_string(),
        ));

        let lexer = Lexer::new(input);
        let tokens = lexer.collect_tokens();

        let err_lexer = Lexer::new(input);
        let errors = err_lexer.collect_errors();
        assert_eq!(errors.len(), 0, "Expected no errors but got: {:?}", errors);

        assert_eq!(
            tokens,
            vec![
                T::Area("AREA".to_string()),
                T::Book("BOOK".to_string()),
                T::Composer("COMPOSER".to_string()),
                T::Discography("DISCOGRAPHY".to_string()),
                T::Filename("FILENAME".to_string()),
                T::Group("GROUP".to_string()),
                T::History("HISTORY".to_string()),
                T::Information("INFO".to_string()),
                T::Notes("NOTES".to_string()),
                T::Origin("ORIGIN".to_string()),
                T::Source("SOURCE".to_string()),
                T::Title("TITLE".to_string()),
                T::Words("WORDS".to_string()),
                T::X("100".to_string()),
                T::Transcription("TRANSCRIPTION".to_string()),
                T::Metre(2, 4),
                T::Metre(5, 8),
                T::DefaultNoteLength(music::FractionalDuration(1, 8)),
                T::KeySignature(
                    music::PitchClass {
                        diatonic_pitch_class: music::DiatonicPitchClass::G,
                        accidental: Some(music::Accidental::Flat),
                    },
                    music::Mode::Major
                ),
            ]
        );

        // Make sure we can lex Windows and Unix line endings.
        let input = &(string_to_vec("T:TITLE\r\nB:BOOK\n".to_string()));

        let tokens = Lexer::new(input).collect_tokens();

        assert_eq!(
            tokens,
            vec![T::Title("TITLE".to_string()), T::Book("BOOK".to_string())]
        );
    }

    /// Errors for reading headers.
    #[test]
    fn header_errs() {
        // Unrecognised start of header.
        match read(Context::new(&(string_to_vec("Y:x\n".to_string())))) {
            LexResult::Error(_, _, LexError::UnexpectedHeaderLine) => {
                assert!(
                    true,
                    "Should get UnexpectedHeaderLine when an unrecognised header line started"
                )
            }
            _ => assert!(false),
        }

        // Good looking header but unrecognised field name.
        match read(Context::new(&(string_to_vec("Y:What\n".to_string())))) {
            LexResult::Error(_, _, LexError::UnexpectedHeaderLine) => {
                assert!(
                    true,
                    "Should get UnexpectedHeaderLine when an unrecognised field type"
                )
            }
            _ => assert!(false),
        }

        // No delimiter (i.e. newline) for field.
        match read(Context::new(&(string_to_vec("T:NeverEnding".to_string())))) {
            LexResult::Error(_, _, LexError::ExpectedDelimiter('\n')) => {
                assert!(
                    true,
                    "Should get ExpectedDelimiter there isn't a newline available"
                )
            }
            _ => assert!(false),
        }

        // Header without colon.
        match read(Context::new(&(string_to_vec("TNoColon".to_string())))) {
            LexResult::Error(_, _, LexError::ExpectedColon) => {
                assert!(
                    true,
                    "Should get ExpectedColon there isn't a newline available"
                )
            }
            _ => assert!(false),
        }


        // Header with unexpected termination.
        match read(Context::new(&(string_to_vec("T".to_string())))) {
            LexResult::Error(_, _, LexError::PrematureEnd(During::Header)) => {
                assert!(
                    true,
                    "Should get ExpectedColon there isn't a newline available"
                )
            }
            _ => assert!(false),
        }
    }

    /// Errors for reading the tune body.
    #[test]
    fn body_errs() {
        // Unexpected character at start of an entity.
        match read(Context::new(&(string_to_vec("x".to_string()))).in_body()) {
            LexResult::Error(_, _, LexError::UnexpectedBodyChar(_)) => {
                assert!(
                    true,
                    "Should get ExpectedColon there isn't a newline available"
                )
            }
            _ => assert!(false),
        }
    }

    /// Tests for simple entities in the tune body.
    #[test]
    fn body_simple_entities() {
        // End of file in tune body.
        match read(Context::new(&(string_to_vec("".to_string()))).in_body()) {
            LexResult::Terminal => {
                assert!(
                    true,
                    "Should lex terminal if end of string in body section."
                )
            }
            _ => assert!(false),
        }

        // End of file in tune body.
        assert_eq!(
            Lexer::new(&(string_to_vec("\n".to_string())))
                .in_body()
                .collect_tokens(),
            vec![T::Newline]
        )

    }

    #[test]
    fn read_until_test() {
        let input = &(string_to_vec(String::from("This\nthat")));
        let context = Context::new(input);

        let result = read_until(context, '\n');

        match result {
            Ok((ctx, value)) => {
                assert_eq!(value, &['T', 'h', 'i', 's']);
                assert_eq!(
                    ctx.i,
                    5,
                    "Next i should be next character after closing delimiter."
                );
            }
            _ => assert!(false, "No result"),
        }
    }

    #[test]
    fn read_number_test() {
        //
        // Match various inputs that terminate at the end of the input.
        //

        // Single digits.
        match read_number(
            Context::new(&(string_to_vec(String::from("0")))),
            NumberRole::UpperTimeSignature,
        ) {
            Ok((_, val)) => assert_eq!(val, 0, "Can read single digit."),
            _ => assert!(false),
        }

        match read_number(
            Context::new(&(string_to_vec(String::from("1")))),
            NumberRole::UpperTimeSignature,
        ) {
            Ok((_, val)) => assert_eq!(val, 1, "Can read single digit."),
            _ => assert!(false),
        }

        // Longer.
        match read_number(
            Context::new(&(string_to_vec(String::from("12345")))),
            NumberRole::UpperTimeSignature,
        ) {
            Ok((_, val)) => assert_eq!(val, 12345),
            _ => assert!(false),
        }

        // Max length.
        match read_number(
            Context::new(&(string_to_vec(String::from("12345678")))),
            NumberRole::UpperTimeSignature,
        ) {
            Ok((_, val)) => assert_eq!(val, 12345678),
            _ => assert!(false),
        }

        //
        // Match various inputs followed by something else.
        //
        match read_number(
            Context::new(&(string_to_vec(String::from("0X")))),
            NumberRole::UpperTimeSignature,
        ) {
            Ok((ctx, val)) => {
                assert_eq!(val, 0, "Can read single digit.");
                assert_eq!(ctx.i, 1, "Index at next character after number.");
            }

            _ => assert!(false),
        }

        match read_number(
            Context::new(&(string_to_vec(String::from("1X")))),
            NumberRole::UpperTimeSignature,
        ) {
            Ok((ctx, val)) => {
                assert_eq!(val, 1, "Can read single digit.");
                assert_eq!(ctx.i, 1, "Index at next character after number.");
            }
            _ => assert!(false),
        }

        // Longer.
        match read_number(
            Context::new(&(string_to_vec(String::from("12345X")))),
            NumberRole::UpperTimeSignature,
        ) {
            Ok((ctx, val)) => {
                assert_eq!(val, 12345, "Can read longer number.");
                assert_eq!(ctx.i, 5, "Index at next character after number.");
            }
            _ => assert!(false),
        }

        // Max length.
        match read_number(
            Context::new(&(string_to_vec(String::from("1234567X")))),
            NumberRole::UpperTimeSignature,
        ) {
            Ok((ctx, val)) => {
                assert_eq!(val, 1234567, "Can read max length number.");
                assert_eq!(ctx.i, 7, "Index at next character after number.");
            }
            _ => assert!(false),
        }

        //
        // Errors
        //

        // Too long to end of input.
        match read_number(
            Context::new(&(string_to_vec(String::from("123456789")))),
            NumberRole::UpperTimeSignature,
        ) {
            Err((_, _, LexError::NumberTooLong(_))) => {
                assert!(true, "Should fail with NumberTooLong")
            }
            _ => assert!(false),
        }

        // No input.
        match read_number(
            Context::new(&(string_to_vec(String::from("")))),
            NumberRole::UpperTimeSignature,
        ) {
            Err((_, _, LexError::ExpectedNumber(_))) => {
                assert!(true, "Should fail with ExpectedNumber")
            }
            _ => assert!(false),
        }

        // Not a number.
        match read_number(
            Context::new(&(string_to_vec(String::from("five")))),
            NumberRole::UpperTimeSignature,
        ) {
            Err((_, _, LexError::ExpectedNumber(_))) => {
                assert!(true, "Should fail with ExpectedNumber")
            }
            _ => assert!(false),
        }

        // NumberRole should be passed through.
        match read_number(
            Context::new(&(string_to_vec(String::from("XX")))),
            NumberRole::UpperTimeSignature,
        ) {
            Err((_, _, LexError::ExpectedNumber(NumberRole::UpperTimeSignature))) => {
                assert!(
                    true,
                    "Correct NumberRole should be passed through to error."
                )
            }
            _ => assert!(false),
        }

        match read_number(
            Context::new(&(string_to_vec(String::from("XX")))),
            NumberRole::LowerTimeSignature,
        ) {
            Err((_, _, LexError::ExpectedNumber(NumberRole::LowerTimeSignature))) => {
                assert!(
                    true,
                    "Correct NumberRole should be passed through to error."
                )
            }
            _ => assert!(false),
        }
    }

    #[test]
    fn lex_metre_test() {
        //
        // Errors
        //

        // Valid time signature but no delimiter means in practice that the field never terminated.
        match lex_metre(Context::new(&(string_to_vec(String::from("C")))), '\n') {
            LexResult::Error(_, _, LexError::PrematureEnd(During::Metre)) => {
                assert!(true, "Should fail with ExpectedMetre")
            }
            _ => assert!(false),
        }

        // Empty time signature.
        match lex_metre(Context::new(&(string_to_vec(String::from("")))), '\n') {
            LexResult::Error(_, _, LexError::PrematureEnd(During::Metre)) => {
                assert!(true, "Should fail with ExpectedMetre")
            }
            _ => assert!(false),
        }

        // Stupid invalid numbers.
        match lex_metre(
            Context::new(&(string_to_vec(String::from("20000000000/1\n")))),
            '\n',
        ) {
            LexResult::Error(_, _, LexError::NumberTooLong(_)) => {
                assert!(true, "Numerator fail with NumberTooLong")
            }
            _ => assert!(false),
        }

        match lex_metre(
            Context::new(&(string_to_vec(String::from("6/80000000000000000\n")))),
            '\n',
        ) {
            LexResult::Error(_, _, LexError::NumberTooLong(_)) => {
                assert!(true, "Denomenator fail with NumberTooLong")
            }
            _ => assert!(false),
        }

        //
        // Shorthand.
        //
        match lex_metre(Context::new(&(string_to_vec(String::from("C\n")))), '\n') {
            LexResult::T(_, tokens) => assert_eq!(tokens, &[T::Metre(4, 4)], "C should be parsed"),
            _ => assert!(false),
        }

        match lex_metre(Context::new(&(string_to_vec(String::from("C|\n")))), '\n') {
            LexResult::T(_, tokens) => assert_eq!(tokens, &[T::Metre(2, 4)], "C should be parsed"),
            _ => assert!(false),
        }

        //
        // Numerical
        //
        match lex_metre(Context::new(&(string_to_vec(String::from("2/4\n")))), '\n') {
            LexResult::T(_, tokens) => {
                assert_eq!(
                    tokens,
                    &[T::Metre(2, 4)],
                    "2/4 time signature should be parsed"
                )
            }
            _ => assert!(false),
        }

        match lex_metre(Context::new(&(string_to_vec(String::from("6/8\n")))), '\n') {
            LexResult::T(_, tokens) => {
                assert_eq!(
                    tokens,
                    &[T::Metre(6, 8)],
                    "6/8 time signature should be parsed"
                )
            }
            _ => assert!(false),
        }

        match lex_metre(
            Context::new(&(string_to_vec(String::from("200/400\n")))),
            '\n',
        ) {
            LexResult::T(_, tokens) => {
                assert_eq!(
                    tokens,
                    &[T::Metre(200, 400)],
                    "Ridiculous but valid time signature should be parsed"
                )
            }
            _ => assert!(false),
        }
    }

    #[test]
    fn read_until_no_delimiter() {
        let input = &(string_to_vec(String::from("This and that")));
        let context = Context::new(input);

        let result = read_until(context, '\n');

        match result {
            Err(_) => assert!(true, "No closing delimiter should result in error."),
            Ok(_) => assert!(false, "No closing delimiter not return a value."),
        }
    }

    // Tests for read()
    #[test]
    fn read_terminal() {
        let empty = &(string_to_vec(String::from(EMPTY)));
        let context = Context::new(empty);

        match read(context) {
            LexResult::Terminal => assert!(true, "Empty results in Terminal character"),
            _ => assert!(false, "Terminal should be returned"),
        }
    }

    #[test]
    fn read_key_note_test() {
        let input = &(string_to_vec(String::from(EMPTY)));
        let context = Context::new(input);
        match read_key_note(context) {
            None => assert!(true, "Read key note empty string gives None"),
            x => assert!(false, "Expected None: {:?}", x),
        }

        let input = &(string_to_vec("C".to_string()));
        let context = Context::new(input);
        match read_key_note(context) {
            Some((_,
                  music::PitchClass {
                      diatonic_pitch_class: music::DiatonicPitchClass::C,
                      accidental: None,
                  })) => assert!(true, "Read diatonic key note only, followed by EOF"),
            x => assert!(false, "Expected diatonic pitch class: {:?}", x),
        }

        let input = &(string_to_vec("C\n".to_string()));
        let ctx = Context::new(input);
        match read_key_note(ctx) {
            Some((new_ctx,
                  music::PitchClass {
                      diatonic_pitch_class: music::DiatonicPitchClass::C,
                      accidental: None,
                  })) => {
                assert_eq!(
                    new_ctx,
                    ctx.skip(1),
                    "Read diatonic key note only, followed by something irrelevant"
                )
            }
            x => assert!(false, "Expected diatonic pitch class: {:?}", x),
        }

        let input = &(string_to_vec("F#\n".to_string()));
        let ctx = Context::new(input);
        match read_key_note(ctx) {
            Some((new_ctx,
                  music::PitchClass {
                      diatonic_pitch_class: music::DiatonicPitchClass::F,
                      accidental: Some(music::Accidental::Sharp),
                  })) => {
                assert_eq!(
                    new_ctx,
                    ctx.skip(2),
                    "Read diatonic key note and accidental, followed by something irrelevant"
                )
            }
            x => assert!(false, "Expected diatonic pitch class: {:?}", x),
        }

        let input = &(string_to_vec("Gf".to_string()));
        let ctx = Context::new(input);
        match read_key_note(ctx) {
            Some((new_ctx,
                  music::PitchClass {
                      diatonic_pitch_class: music::DiatonicPitchClass::G,
                      accidental: Some(music::Accidental::Flat),
                  })) => {
                assert_eq!(
                    new_ctx,
                    ctx.skip(2),
                    "Read diatonic key note and accidental, followed by EOF"
                )
            }
            x => assert!(false, "Expected diatonic pitch class: {:?}", x),
        }
    }

    #[test]
    fn read_mode_test() {

        // Case insensitive long form, ignoring spaces.
        // Test both, to ensure that the short one doesn't get matched, leaving ctx dangling in the
        // middle of a word.
        let input = &(string_to_vec("major".to_string()));
        let ctx = Context::new(input);
        match read_mode(ctx) {
            Some((new_ctx, music::Mode::Major)) => {
                assert_eq!(new_ctx, ctx.skip(5), "Read normal mode works")
            }
            x => assert!(false, "Expected mode got: {:?}", x),
        };

        let input = &(string_to_vec("MaJoR".to_string()));
        let ctx = Context::new(input);
        match read_mode(ctx) {
            Some((new_ctx, music::Mode::Major)) => {
                assert_eq!(
                    new_ctx,
                    ctx.skip(5),
                    "Read normal mode works, case insensitive"
                )
            }
            x => assert!(false, "Expected mode got: {:?}", x),
        }

        let input = &(string_to_vec("     MaJoR".to_string()));
        let ctx = Context::new(input);
        match read_mode(ctx) {
            Some((new_ctx, music::Mode::Major)) => {
                assert_eq!(
                    new_ctx,
                    ctx.skip(10),
                    "Read normal mode works and skips leading whitespace, case insensitive"
                )
            }
            x => assert!(false, "Expected mode got: {:?}", x),
        }

        // Case insensitive short form, ignoring spaces.
        let input = &(string_to_vec("maj".to_string()));
        let ctx = Context::new(input);
        match read_mode(ctx) {
            Some((new_ctx, music::Mode::Major)) => {
                assert_eq!(new_ctx, ctx.skip(3), "Read normal mode works")
            }
            x => assert!(false, "Expected mode got: {:?}", x),
        };

        let input = &(string_to_vec("MaJ".to_string()));
        let ctx = Context::new(input);
        match read_mode(ctx) {
            Some((new_ctx, music::Mode::Major)) => {
                assert_eq!(
                    new_ctx,
                    ctx.skip(3),
                    "Read normal mode works and skips leading whitespace, case insensitive"
                )
            }
            x => assert!(false, "Expected mode got: {:?}", x),
        }

        let input = &(string_to_vec("   MaJ".to_string()));
        let ctx = Context::new(input);
        match read_mode(ctx) {
            Some((new_ctx, music::Mode::Major)) => {
                assert_eq!(
                    new_ctx,
                    ctx.skip(6),
                    "Read short form mode works, case insensitive, skipping whitespace"
                )
            }
            x => assert!(false, "Expected mode got: {:?}", x),
        }

    }

    #[test]
    fn read_n_time_test() {
        let input = &(string_to_vec("[1".to_string()));
        let ctx = Context::new(input);
        match read_n_time(ctx) {
            (ctx, Some(n_time)) => assert_eq!(n_time, 1),
            x => assert!(false, "Expected ntime got: {:?}", x),
        }

        // Bracket is optional.
        let input = &(string_to_vec("2".to_string()));
        let ctx = Context::new(input);
        match read_n_time(ctx) {
            (ctx, Some(n_time)) => assert_eq!(n_time, 2),
            x => assert!(false, "Expected ntime got: {:?}", x),
        }
    }

    #[test]
    fn lex_barline_test() {
        let input = &(string_to_vec("|".to_string()));
        let ctx = Context::new(input);
        match lex_barline(ctx) {
            LexResult::T(_, tokens) => {
                assert_eq!(
                    tokens,
                    &[
                        T::Barline(music::Barline {
                            repeat_before: false,
                            single: true,
                            repeat_after: false,
                            n_time: None,
                        }),
                    ]
                )
            }

            x => assert!(false, "Expected barline got: {:?}", x),
        }

        let input = &(string_to_vec("|:".to_string()));
        let ctx = Context::new(input);
        match lex_barline(ctx) {
            LexResult::T(_, tokens) => {
                assert_eq!(
                    tokens,
                    &[
                        T::Barline(music::Barline {
                            repeat_before: false,
                            single: true,
                            repeat_after: true,
                            n_time: None,
                        }),
                    ]
                )
            }

            x => assert!(false, "Expected barline got: {:?}", x),
        }

        let input = &(string_to_vec(":|".to_string()));
        let ctx = Context::new(input);
        match lex_barline(ctx) {
            LexResult::T(_, tokens) => {
                assert_eq!(
                    tokens,
                    &[
                        T::Barline(music::Barline {
                            repeat_before: true,
                            single: true,
                            repeat_after: false,
                            n_time: None,
                        }),
                    ]
                )
            }

            x => assert!(false, "Expected barline got: {:?}", x),
        }

        let input = &(string_to_vec(":|:".to_string()));
        let ctx = Context::new(input);
        match lex_barline(ctx) {
            LexResult::T(_, tokens) => {
                assert_eq!(
                    tokens,
                    &[
                        T::Barline(music::Barline {
                            repeat_before: true,
                            single: true,
                            repeat_after: true,
                            n_time: None,
                        }),
                    ]
                )
            }

            x => assert!(false, "Expected barline got: {:?}", x),
        }

        let input = &(string_to_vec("::".to_string()));
        let ctx = Context::new(input);
        match lex_barline(ctx) {
            LexResult::T(_, tokens) => {
                assert_eq!(
                    tokens,
                    &[
                        T::Barline(music::Barline {
                            repeat_before: true,
                            single: true,
                            repeat_after: true,
                            n_time: None,
                        }),
                    ]
                )
            }

            x => assert!(false, "Expected barline got: {:?}", x),
        }

        let input = &(string_to_vec("||".to_string()));
        let ctx = Context::new(input);
        match lex_barline(ctx) {
            LexResult::T(_, tokens) => {
                assert_eq!(
                    tokens,
                    &[
                        T::Barline(music::Barline {
                            repeat_before: false,
                            single: false,
                            repeat_after: false,
                            n_time: None,
                        }),
                    ]
                )
            }

            x => assert!(false, "Expected barline got: {:?}", x),
        }
    }


    ///
    /// N-time repeat bars
    ///
    #[test]
    fn lex_barline_n_time_test() {
        let input = &(string_to_vec("|[1".to_string()));
        let ctx = Context::new(input);
        match lex_barline(ctx) {
            LexResult::T(_, tokens) => {
                assert_eq!(
                    tokens,
                    &[
                        T::Barline(music::Barline {
                            repeat_before: false,
                            single: true,
                            repeat_after: false,
                            n_time: Some(1),
                        }),
                    ]
                )
            }

            x => assert!(false, "Expected barline got: {:?}", x),
        }

        // Bracket is optional.
        let input = &(string_to_vec("|1".to_string()));
        let ctx = Context::new(input);
        match lex_barline(ctx) {
            LexResult::T(_, tokens) => {
                assert_eq!(
                    tokens,
                    &[
                        T::Barline(music::Barline {
                            repeat_before: false,
                            single: true,
                            repeat_after: false,
                            n_time: Some(1),
                        }),
                    ]
                )
            }

            x => assert!(false, "Expected barline got: {:?}", x),
        }

        let input = &(string_to_vec(":|[2".to_string()));
        let ctx = Context::new(input);
        match lex_barline(ctx) {
            LexResult::T(_, tokens) => {
                assert_eq!(
                    tokens,
                    &[
                        T::Barline(music::Barline {
                            repeat_before: true,
                            single: true,
                            repeat_after: false,
                            n_time: Some(2),
                        }),
                    ]
                )
            }

            x => assert!(false, "Expected barline got: {:?}", x),
        }

        let input = &(string_to_vec(":|2".to_string()));
        let ctx = Context::new(input);
        match lex_barline(ctx) {
            LexResult::T(_, tokens) => {
                assert_eq!(
                    tokens,
                    &[
                        T::Barline(music::Barline {
                            repeat_before: true,
                            single: true,
                            repeat_after: false,
                            n_time: Some(2),
                        }),
                    ]
                )
            }

            x => assert!(false, "Expected barline got: {:?}", x),
        }

    }

    #[test]
    fn starts_with_insensitive_eager_test() {
        let input = &(string_to_vec("".to_string()));
        let ctx = Context::new(input);
        match ctx.starts_with_insensitive_eager(&[]) {
            (new_ctx, true) => assert_eq!(ctx, new_ctx, "Empty string starts with empty string"),
            _ => assert!(false, "Expected match"),
        }

        let input = &(string_to_vec("hello".to_string()));
        let ctx = Context::new(input);
        match ctx.starts_with_insensitive_eager(&[]) {
            (new_ctx, true) => assert_eq!(ctx, new_ctx, "Some string starts with empty string"),
            _ => assert!(false, "Expected match"),
        }

        let input = &(string_to_vec("hello world".to_string()));
        let ctx = Context::new(input);
        match ctx.starts_with_insensitive_eager(&['h', 'e', 'l', 'l', 'o']) {
            (new_ctx, true) => {
                assert_eq!(
                    ctx.skip(5),
                    new_ctx,
                    "Some string starts with its prefix and skip that lenght"
                )
            }
            _ => assert!(false, "Expected match"),
        }

        let input = &(string_to_vec("hello world".to_string()));
        let ctx = Context::new(input);
        match ctx.starts_with_insensitive_eager(&['H', 'e', 'L', 'l', 'O']) {
            (new_ctx, true) => {
                assert_eq!(
                    ctx.skip(5),
                    new_ctx,
                    "Some string starts with its prefix different case and skip that lenght"
                )
            }
            _ => assert!(false, "Expected match"),
        }

        let input = &(string_to_vec("hello world".to_string()));
        let ctx = Context::new(input);
        match ctx.starts_with_insensitive_eager(&['h', 'e', 'l', 'l', 'X']) {
            (new_ctx, false) => {
                assert_eq!(
                    ctx,
                    new_ctx,
                    "Some string doesn't start with prefix that has non-matching char"
                )
            }
            _ => assert!(false, "Expected false"),
        }


        let input = &(string_to_vec("hell".to_string()));
        let ctx = Context::new(input);
        match ctx.starts_with_insensitive_eager(&['h', 'e', 'l', 'l', 'o']) {
            (new_ctx, false) => {
                assert_eq!(ctx, new_ctx, "Prefix longer than context returns false.")
            }
            _ => assert!(false, "Expected false"),
        }
    }

    #[test]

    fn skip_optional_prefix_test() {
        let input = &(string_to_vec("".to_string()));
        let ctx = Context::new(input);
        assert_eq!(
            ctx.skip_optional_prefix(&[]).i,
            0,
            "Offset is not incremented for empty prefix of empty"
        );
        let ctx = Context::new(input);
        assert_eq!(
            ctx.skip_optional_prefix(&['X']).i,
            0,
            "Offset is not incremented for some optional prefix of empty"
        );

        let input = &(string_to_vec("hello".to_string()));
        let ctx = Context::new(input);
        assert_eq!(
            ctx.skip_optional_prefix(&[]).i,
            0,
            "Offset is not incremented for empty prefix of some"
        );
        let ctx = Context::new(input);
        assert_eq!(
            ctx.skip_optional_prefix(&['h', 'e']).i,
            2,
            "Offset is incremented for some prefix of some."
        );
    }

}
