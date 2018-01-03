///! ABC Lexer
///! Transform strings of ABC into a sequence of lexed tokens.
///! This accepts a String and returns newly allocated strings that have an independent lifetime to
///! the supplied string.

/// Which bit of the tune are we in?
#[derive(Debug, PartialEq, PartialOrd, Clone)]
enum TuneContext {
    Header,
    Body,
}

/// Context required to lex an ABC String.
/// Context object is immutable for simpler state and testing.
#[derive(Debug, PartialEq, PartialOrd, Clone)]
struct Context<'a> {
    /// The ABC tune content as a vector of potentially multibyte characters.
    /// Stored as a slice of chars so we can peek.
    c: &'a [char],

    // Length of string.
    l: usize,

    // The current index of the string during lexing.
    i: usize,

    tune_context: TuneContext,
}

impl<'a> Context<'a> {
    fn new(c: &'a [char]) -> Context<'a> {

        let l = c.len();

        Context {
            c,
            l,
            i: 0,
            tune_context: TuneContext::Header,
        }
    }

    /// Are there this many characters available?
    fn has(&self, chars: usize) -> bool {
        self.i + chars <= self.l
    }

    /// Move to body state.
    fn in_body(&self) -> Context<'a> {
        Context {
            tune_context: TuneContext::Body,
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
fn read_until<'a>(
    ctx: Context<'a>,
    delimiter: char,
) -> Result<(Context<'a>, &'a [char]), Context<'a>> {
    // Find the index of the first delimiter.
    let delimiter_char = ctx.c[ctx.i..].iter().enumerate().take_while(
        |&(_, c)| c != &delimiter,
    );

    if let Some((length, _)) = delimiter_char.last() {
        // If we reached the end of the input and there was no delimiter, error.
        if ctx.i + length + 1 >= ctx.l || ctx.c[ctx.i + length + 1] != delimiter {
            Err(ctx)
        } else {
            // Retrieve as subslice of original.
            let value = &ctx.c[ctx.i..ctx.i + length + 1];
            Ok((
                Context {
                    i: ctx.i + length + 2,
                    ..ctx
                },
                value,
            ))
        }
    } else {
        // If there was no delimiter, end of story.
        // Return the context in case it's needed for error reporting.
        Err(ctx)
    }
}

/// Types of errors. These should be as specific as possible to give the best help.
/// Avoiding generic 'expected char' type values.
#[derive(Debug)]
enum LexError {
    /// We expected to find a delimiter at some point after the current position but couldn't.
    ExpectedDelimiter(char),

    /// We expected a field type (e.g. "T") but didn't get one.
    ExpectedFieldType,

    /// We expected to find a colon character.
    ExpectedColon,

    /// In the tune header, we found a start of line that we couldn't recognise.
    UnexpectedHeaderLine,

    /// In the tune body, where we expect the start of a token, we got a character we didn't expect.
    UnexpectedBodyChar,

    /// Feature not implemented yet.
    /// Should have no tests for this.
    /// TODO remove this when feautre complete.
    UnimplementedError,
}

/// A glorified Option type that allows encoding errors.
#[derive(Debug)]
enum LexResult<'a> {
    /// Token. Shortened as it's used a lot.
    T(Context<'a>, T),
    Error(Context<'a>, LexError),
}

/// ABC Token.
/// Shortened as it's used a lot.
#[derive(Debug, PartialEq, PartialOrd)]
enum T {
    Terminal,
    Newline,

    // A useless character.
    Skip,

    // Text fields
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
}

/// Try to read a single T and return a new context.
fn read(ctx: Context) -> LexResult {
    // Need to peek 1 ahead. If we can't, we'are at the end.
    if !ctx.has(1) {
        return LexResult::T(ctx, T::Terminal);
    }

    let first_char = ctx.c[ctx.i];

    match ctx.tune_context {
        TuneContext::Header => {
            match first_char {
                // Text headers.
                'A' | 'B' | 'C' | 'D' | 'F' | 'G' | 'H' | 'I' | 'N' | 'O' | 'R' | 'S' | 'T' |
                'W' | 'X' | 'Z' => {
                    if ctx.has(2) && ctx.c[ctx.i + 1] == ':' {
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
                                    _ => return LexResult::Error(ctx, LexError::ExpectedFieldType),
                                }
                            }
                            Err(ctx) => {
                                return LexResult::Error(ctx, LexError::ExpectedDelimiter('\n'))
                            }
                        }
                    } else {
                        return LexResult::Error(ctx, LexError::ExpectedColon);
                    }
                }

                // Key signature.
                // TODO remember to switch tune context.
                'K' => return LexResult::Error(ctx, LexError::UnimplementedError),

                // Default note length.
                'L' => return LexResult::Error(ctx, LexError::UnimplementedError),

                // Metre.
                'M' => return LexResult::Error(ctx, LexError::UnimplementedError),

                // Parts.
                'P' => return LexResult::Error(ctx, LexError::UnimplementedError),

                // Tempo
                'Q' => return LexResult::Error(ctx, LexError::UnimplementedError),

                // Anything else in the header is unrecognised.
                _ => return LexResult::Error(ctx, LexError::UnexpectedHeaderLine),

            };
        }

        TuneContext::Body => {
            match first_char {
                // Better to lex these individually so that we account for each character,
                // then ignore the Skips later.
                '\n' => return LexResult::T(ctx.skip(1), T::Newline),
                '\r' => return LexResult::T(ctx.skip(1), T::Skip),

                // TODO all tune body entities.
                _ => return LexResult::Error(ctx, LexError::UnexpectedBodyChar),
            }
        }
    };
}


/// A stateful lexer for an ABC string.
/// Implements Iterator.
struct Lexer<'a> {
    // content: &'a[char],
    context: Context<'a>,
    error: Option<(Context<'a>, LexError)>,
}

impl<'a> Lexer<'a> {
    fn new(content: &'a [char]) -> Lexer<'a> {
        let context = Context::new(&content);

        // The error we encountered.
        // Becuase iteration stops at the first error, we only need to store one.
        let error = None;

        Lexer {
            // content,
            context,
            error,
        }
    }

    // Skip into the body. For testing only.
    fn in_body(mut self) -> Lexer<'a> {
        self.context = self.context.in_body();
        self
    }
}


impl<'a> Iterator for Lexer<'a> {
    type Item = T;

    fn next(&mut self) -> Option<T> {
        // Take a temporary clone of self.context so it can be consumed.
        // TODO could read() work with a ref?
        match read(self.context.clone()) {
            LexResult::T(new_context, token) => {
                self.context = new_context;

                match token {
                    // Terminal token means stop iterating.
                    T::Terminal => None,

                    // Anything else, return and keep iterating.
                    _ => Some(token),
                }
            }
            LexResult::Error(new_context, error) => {
                // An error stops iteration.
                self.error = Some((new_context, error));
                None
            }
        }
    }
}

fn string_to_vec(input: String) -> Vec<char> {
    input.chars().collect::<Vec<char>>()
}

#[cfg(test)]
mod tests {
    use super::*;

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
"
                .to_string(),
        ));

        let tokens = Lexer::new(input).collect::<Vec<T>>();

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
            ]
        );

        // Make sure we can lex Windows and Unix line endings.
        let input = &(string_to_vec("T:TITLE\r\nB:BOOK\n".to_string()));

        let tokens = Lexer::new(input).collect::<Vec<T>>();

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
            LexResult::Error(_, LexError::UnexpectedHeaderLine) => {
                assert!(
                    true,
                    "Should get UnexpectedHeaderLine when an unrecognised header line started"
                )
            }
            _ => assert!(false),
        }

        // Good looking header but unrecognised field name.
        match read(Context::new(&(string_to_vec("Y:What\n".to_string())))) {
            LexResult::Error(_, LexError::UnexpectedHeaderLine) => {
                assert!(
                    true,
                    "Should get UnexpectedHeaderLine when an unrecognised field type"
                )
            }
            _ => assert!(false),
        }

        // No delimiter (i.e. newline) for field.
        match read(Context::new(&(string_to_vec("T:NeverEnding".to_string())))) {
            LexResult::Error(_, LexError::ExpectedDelimiter('\n')) => {
                assert!(
                    true,
                    "Should get ExpectedDelimiter there isn't a newline available"
                )
            }
            _ => assert!(false),
        }

        // Header without colon.
        match read(Context::new(&(string_to_vec("TNoColon".to_string())))) {
            LexResult::Error(_, LexError::ExpectedColon) => {
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
            LexResult::Error(_, LexError::UnexpectedBodyChar) => {
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
            Lexer::new(&(string_to_vec("\r\n".to_string())))
                .in_body()
                .collect::<Vec<T>>(),
            vec![T::Skip, T::Newline]
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
