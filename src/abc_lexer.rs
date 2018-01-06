///! ABC Lexer
///! Transform strings of ABC into a sequence of lexed tokens.
///! This accepts a String and returns newly allocated strings that have an independent lifetime to
///! the supplied string.
///! When lex_* and read_* functions return errors, they should leave the context in the most
///! helpful state so that the next token has a good chance at understanding it.
///! e.g. don't bomb out half way through the time signature.

/// Which bit of the tune are we in?
#[derive(Debug, PartialEq, PartialOrd, Clone, Copy)]
enum TuneSection {
    Header,
    Body,
}

/// Context required to lex an ABC String.
/// Context object is immutable for simpler state and testing.
#[derive(Debug, PartialEq, PartialOrd, Clone, Copy)]
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
                LexResult::T(ctx, T::Metre(4, 4))
            } else if content == &['C', '|'] {
                LexResult::T(ctx, T::Metre(2, 4))
            } else {
                // It's a numerical metre.
                match read_number(ctx, NumberRole::UpperTimeSignature) {
                    Err((_, offset, err)) => LexResult::Error(whole_line_ctx, offset, err),
                    Ok((ctx, numerator)) => {
                        if !(ctx.has(1) && ctx.c[ctx.i] == '/') {
                            LexResult::Error(ctx, ctx.i, LexError::ExpectedSlashInMetre)
                        } else {

                            // Skip slash.
                            let ctx = ctx.skip(1);

                            match read_number(ctx, NumberRole::LowerTimeSignature) {
                                Err((_, offset, err)) => {
                                    LexResult::Error(whole_line_ctx, offset, err)
                                }
                                Ok((ctx, denomenator)) => {
                                    // Skip one character for the delimiter.
                                    LexResult::T(ctx.skip(1), T::Metre(numerator, denomenator))
                                }
                            }
                        }

                    }

                }

            }
        }
    }
}

// The activity we were undertaking at the time when something happened.
#[derive(Debug, PartialEq, PartialOrd, Clone)]
pub enum During {
    Metre,
}

#[derive(Debug, PartialEq, PartialOrd, Clone)]
pub enum NumberRole {
    UpperTimeSignature,
    LowerTimeSignature,
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
                              and quote number ");
                buf.push_str(&ident.to_string());
                buf.push_str("and I'll see if I can fix it.");
            }
        }
    }
}

/// A glorified Option type that allows encoding errors.
#[derive(Debug, PartialEq, PartialOrd, Clone)]
pub enum LexResult<'a> {
    /// Token. Shortened as it's used a lot.
    T(Context<'a>, T),
    /// Error contains a context and an offset of where the error occurred.
    /// The context's offset is used to resume, and should point to the end of the troublesome bit.
    /// The error's offset indidates where the error happened, i.e. the start of the bother.
    Error(Context<'a>, usize, LexError),
}

/// ABC Token.
/// Shortened as it's used a lot.
#[derive(Debug, PartialEq, PartialOrd, Clone)]
pub enum T {
    Terminal,
    Newline,

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
}

/// Try to read a single T and return a new context.
/// Note that there's a lot of aliasing of ctx in nested matches.
fn read(ctx: Context) -> LexResult {
    // Need to peek 1 ahead. If we can't, we'are at the end.
    if !ctx.has(1) {
        return LexResult::T(ctx, T::Terminal);
    }

    let first_char = ctx.c[ctx.i];

    match ctx.tune_section {
        TuneSection::Header => {
            match first_char {
                // Text headers.
                'A' | 'B' | 'C' | 'D' | 'F' | 'G' | 'H' | 'I' | 'N' | 'O' | 'R' | 'S' | 'T' |
                'W' | 'X' | 'Z' => {
                    if !(ctx.has(2) && ctx.c[ctx.i + 1] == ':') {
                        return LexResult::Error(ctx, ctx.i + 1, LexError::ExpectedColon);
                    } else {
                        match read_until(ctx, '\n') {
                            Ok((ctx, chars)) => {
                                // Skip field label and colon.
                                let value: String = chars.iter().skip(2).collect();

                                // Strip whitespace including leading space and trailing newline
                                let value = value.trim().to_string();

                                match first_char {
                                    'A' => return LexResult::T(ctx, T::Area(value)),
                                    'B' => return LexResult::T(ctx, T::Book(value)),
                                    'C' => return LexResult::T(ctx, T::Composer(value)),
                                    'D' => return LexResult::T(ctx, T::Discography(value)),
                                    'F' => return LexResult::T(ctx, T::Filename(value)),
                                    'G' => return LexResult::T(ctx, T::Group(value)),
                                    'H' => return LexResult::T(ctx, T::History(value)),
                                    'I' => return LexResult::T(ctx, T::Information(value)),
                                    'N' => return LexResult::T(ctx, T::Notes(value)),
                                    'O' => return LexResult::T(ctx, T::Origin(value)),
                                    'S' => return LexResult::T(ctx, T::Source(value)),
                                    'T' => return LexResult::T(ctx, T::Title(value)),
                                    'W' => return LexResult::T(ctx, T::Words(value)),
                                    'X' => return LexResult::T(ctx, T::X(value)),
                                    'Z' => return LexResult::T(ctx, T::Transcription(value)),

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
                            Err(ctx) => {
                                return LexResult::Error(
                                    ctx,
                                    ctx.i,
                                    LexError::ExpectedDelimiter('\n'),
                                )
                            }
                        }
                    }
                }

                // Non-text headers.
                // Grouped for handling code.
                'K' | 'L' | 'M' | 'P' | 'Q' => {
                    if !(ctx.has(2) && ctx.c[ctx.i + 1] == ':') {
                        return LexResult::Error(ctx, ctx.i, LexError::ExpectedColon);
                    } else {
                        let start_offset = ctx.i;

                        // Skip colon and field.
                        let ctx = ctx.skip(2);

                        match first_char {
                            // Key signature.
                            // TODO remember to switch tune context.
                            'K' => {
                                // K signals a switch to the body section.
                                let ctx = ctx.in_body();

                                return LexResult::Error(
                                    ctx,
                                    start_offset,
                                    LexError::UnimplementedError(1),
                                )
                            }

                            // Default note length.
                            'L' => {
                                return LexResult::Error(
                                    ctx,
                                    start_offset,
                                    LexError::UnimplementedError(2),
                                )
                            }

                            // Metre.
                            'M' => return lex_metre(ctx, '\n'),

                            // Parts.
                            'P' => {
                                return LexResult::Error(
                                    ctx,
                                    start_offset,
                                    LexError::UnimplementedError(3),
                                )
                            }

                            // Tempo
                            'Q' => {
                                return LexResult::Error(
                                    ctx,
                                    start_offset,
                                    LexError::UnimplementedError(4),
                                )
                            }

                            // This can only happen if the above cases get out of sync.
                            _ => {
                                return LexResult::Error(
                                    ctx,
                                    start_offset,
                                    LexError::ExpectedFieldType(first_char),
                                )
                            }
                        }
                    }
                }

                // Anything else in the header is unrecognised.
                _ => return LexResult::Error(ctx, ctx.i, LexError::UnexpectedHeaderLine),
            };
        }

        TuneSection::Body => {
            match first_char {
                '\n' => return LexResult::T(ctx.skip(1), T::Newline),

                // TODO all tune body entities.
                _ => return LexResult::Error(ctx, ctx.i, LexError::UnexpectedBodyChar(first_char)),
            }
        }
    };
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
    /// For testing. A real consumer should take account of errors!
    #[cfg(test)]
    fn collect_tokens(self) -> Vec<T> {
        self.filter_map(|x| match x {
            LexResult::T(_, token) => Some(token),
            LexResult::Error(_, _, _) => None,
        }).collect::<Vec<T>>()
    }

    fn collect_errors(self) -> Vec<(Context<'a>, usize, LexError)> {
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
            LexResult::T(context, T::Terminal) => {
                self.context = context;
                None
            }

            // If it's an error, return it and set the flag.
            LexResult::Error(context, offset, error) => {
                self.context = context.clone();
                self.error = Some(error.clone());
                Some(LexResult::Error(context, offset, error))
            }

            // Otherwise it's a token.
            LexResult::T(context, token) => {
                self.context = context.clone();
                Some(LexResult::T(context, token))
            }
        }
    }
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
        let butterfly = &(string_to_vec(String::from(BUTTERFLY)));

        let context = Context::new(butterfly);
        assert_eq!(
            context.has(0),
            true,
            "A full string has at least 0 more characters"
        );
        assert_eq!(
            context.has(1),
            true,
            "A full string has at least 1 more characters"
        );
        assert_eq!(
            context.has(2),
            true,
            "A full string has at least 2 more characters"
        );

        let x = &(String::from("x").chars().collect::<Vec<char>>()[..]);
        let context = Context::new(x);

        assert_eq!(
            context.has(0),
            true,
            "A one-length string has at least 0 more characters"
        );
        assert_eq!(
            context.has(1),
            true,
            "A one-length string has at least 1 more characters"
        );
        assert_eq!(
            context.has(2),
            false,
            "A one-length string has NOT got at least 2 more characters"
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
            LexResult::T(_, ref token) => assert_eq!(token, &T::Title("Title".to_string())),
            _ => assert!(false),
        }

        match all_results[1] {
            LexResult::Error(_, _, LexError::ExpectedNumber(_)) => assert!(true),
            _ => assert!(false),
        }

        match all_results[2] {
            LexResult::T(_, ref token) => assert_eq!(token, &T::Composer("Composer".to_string())),
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

    #[test]
    fn read_text_headers_test() {
        let input = &(string_to_vec(
            "A:AREA
B:BOOK
C:COMPOSER
D:DISCOGRAPHY
F:FILENAME
G:GROUP
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
"
                .to_string(),
        ));

        let lexer = Lexer::new(input);
        let tokens = lexer.collect_tokens();

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
            LexResult::T(_, T::Terminal) => {
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
            LexResult::T(_, T::Metre(4, 4)) => assert!(true, "C should be parsed"),
            _ => assert!(false),
        }

        match lex_metre(Context::new(&(string_to_vec(String::from("C|\n")))), '\n') {
            LexResult::T(_, T::Metre(2, 4)) => assert!(true, "C should be parsed"),
            _ => assert!(false),
        }

        //
        // Numerical
        //
        match lex_metre(Context::new(&(string_to_vec(String::from("2/4\n")))), '\n') {
            LexResult::T(_, T::Metre(2, 4)) => assert!(true, "2/4 time signature should be parsed"),
            _ => assert!(false),
        }

        match lex_metre(Context::new(&(string_to_vec(String::from("6/8\n")))), '\n') {
            LexResult::T(_, T::Metre(6, 8)) => assert!(true, "6/8 time signature should be parsed"),
            _ => assert!(false),
        }

        match lex_metre(
            Context::new(&(string_to_vec(String::from("200/400\n")))),
            '\n',
        ) {
            LexResult::T(_, T::Metre(200, 400)) => {
                assert!(true, "Ridiculous but valid time signature should be parsed")
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
            LexResult::T(_, T::Terminal) => assert!(true, "Empty results in Terminal character"),
            _ => assert!(false, "Terminal should be returned"),
        }
    }
}



/// Parse an ABC input, return nicely formatted error message and number of lex errors.
pub fn format_error_message<'a>(input: &[char], all_errors: Vec<(Context<'a>, usize, LexError)>) -> (usize, u32, String) {
    const ABC_PREFIX: &str = "  | ";
    const ERR_PREFIX: &str = "  > ";

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
            let mut first_line = true;
            for error_line in error_index.iter() {
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
                                    buf.push(if error_line == error_char {
                                        if first_line {
                                            first_line = false;
                                            '^'
                                        } else {
                                            '|'
                                        }
                                    } else {
                                        '-'
                                    });
                                    indent += 1;

                                    buf.push_str(&"-- ");
                                    indent += 3;

                                    error.format(indent, &mut buf);

                                    // If we reached the target error, don't keep scanning the line.
                                    break;

                                }
                            }
                        }
                        buf.push('\n');
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