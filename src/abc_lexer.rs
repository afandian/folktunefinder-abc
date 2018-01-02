///! ABC Lexer
///! Transform strings of ABC into a sequence of lexed tokens.
///! This accepts a String and returns newly allocated strings that have an independent lifetime to
///! the supplied string.

/// Which bit of the tune are we in?
#[derive(Debug, PartialEq, PartialOrd)]
enum TuneContext {
    Header,
    Body,
}

/// Context required to parse an ABC String.
/// Context object is immutable for simpler state and testing.
#[derive(Debug, PartialEq, PartialOrd)]
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

    /// Copy with the new field delimiter.
    fn with_tune_context(&self, new_val: TuneContext) -> Context<'a> {
        Context {
            tune_context: new_val,
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
fn read_until<'a>(ctx: Context<'a>, delimiter: char) -> Option<(Context<'a>, &'a [char])> {
    // Find the index of the first delimiter.
    let delimiter_char = ctx.c[ctx.i..].iter().enumerate().take_while(
        |&(_, c)| c != &delimiter,
    );

    if let Some((length, _)) = delimiter_char.last() {
        let value = &ctx.c[ctx.i..ctx.i + length + 1];
        return Some((
            Context {
                i: ctx.i + length + 2,
                ..ctx
            },
            value,
        ));
    }

    // If there was no delimiter, end of story.
    return None;
}

#[derive(Debug, PartialEq, PartialOrd)]
enum AbcToken {
    Terminal,
    Newline,

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

/// Try to read a single AbcToken and return a new context.
fn read(ctx: Context) -> Option<(Context, AbcToken)> {
    // Need to peek 1 ahead. If we can't, we'are at the end.
    if !ctx.has(1) {
        return Some((ctx, AbcToken::Terminal));
    }

    match ctx.tune_context {
        TuneContext::Header => {
            let first_char = ctx.c[ctx.i];

            match first_char {
                '\n' => return Some((ctx.skip(1), AbcToken::Newline)),

                'A' | 'B' | 'C' | 'D' | 'F' | 'G' | 'H' | 'I' | 'N' | 'O' | 'R' | 'S' | 'T' |
                'W' | 'X' | 'Z' => {
                    if ctx.has(2) && ctx.c[ctx.i + 1] == ':' {
                        match read_until(ctx, '\n') {
                            Some((ctx, chars)) => {
                                // Skip field label and colon.
                                let value = chars.iter().skip(2).collect();

                                match first_char {
                                    'A' => return Some((ctx, AbcToken::Area(value))),
                                    'B' => return Some((ctx, AbcToken::Book(value))),
                                    'C' => return Some((ctx, AbcToken::Composer(value))),
                                    'D' => return Some((ctx, AbcToken::Discography(value))),
                                    'F' => return Some((ctx, AbcToken::Filename(value))),
                                    'G' => return Some((ctx, AbcToken::Group(value))),
                                    'H' => return Some((ctx, AbcToken::History(value))),
                                    'I' => return Some((ctx, AbcToken::Information(value))),
                                    'N' => return Some((ctx, AbcToken::Notes(value))),
                                    'O' => return Some((ctx, AbcToken::Origin(value))),
                                    'S' => return Some((ctx, AbcToken::Source(value))),
                                    'T' => return Some((ctx, AbcToken::Title(value))),
                                    'W' => return Some((ctx, AbcToken::Words(value))),
                                    'X' => return Some((ctx, AbcToken::X(value))),
                                    'Z' => return Some((ctx, AbcToken::Transcription(value))),
                                    _ => (),
                                }
                            }
                            _ => (),
                        }
                    }

                }

                _ => (),

            };
        }

        TuneContext::Body => {
            // TODO

        }
    };


    return None;
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
        let input = &(string_to_vec(String::from(
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
Z:TRANSCRIPTION",
        )));

        let context = Context::new(input);

        let (context, result) = read(context).unwrap();
        assert_eq!(result, AbcToken::Area(String::from("AREA")));

        let (context, result) = read(context).unwrap();
        assert_eq!(result, AbcToken::Book(String::from("BOOK")));

        let (context, result) = read(context).unwrap();
        assert_eq!(result, AbcToken::Composer(String::from("COMPOSER")));

        let (context, result) = read(context).unwrap();
        assert_eq!(result, AbcToken::Discography(String::from("DISCOGRAPHY")));

        let (context, result) = read(context).unwrap();
        assert_eq!(result, AbcToken::Filename(String::from("FILENAME")));

        let (context, result) = read(context).unwrap();
        assert_eq!(result, AbcToken::Group(String::from("GROUP")));

        let (context, result) = read(context).unwrap();
        assert_eq!(result, AbcToken::History(String::from("HISTORY")));

        let (context, result) = read(context).unwrap();
        assert_eq!(result, AbcToken::Information(String::from("INFO")));

        let (context, result) = read(context).unwrap();
        assert_eq!(result, AbcToken::Notes(String::from("NOTES")));

        let (context, result) = read(context).unwrap();
        assert_eq!(result, AbcToken::Origin(String::from("ORIGIN")));

        let (context, result) = read(context).unwrap();
        assert_eq!(result, AbcToken::Source(String::from("SOURCE")));

        let (context, result) = read(context).unwrap();
        assert_eq!(result, AbcToken::Title(String::from("TITLE")));

        let (context, result) = read(context).unwrap();
        assert_eq!(result, AbcToken::Words(String::from("WORDS")));

        let (context, result) = read(context).unwrap();
        assert_eq!(result, AbcToken::X(String::from("100")));

        let (context, result) = read(context).unwrap();
        assert_eq!(
            result,
            AbcToken::Transcription(String::from("TRANSCRIPTION"))
        );

        let (context, result) = read(context).unwrap();
        assert_eq!(result, AbcToken::Terminal);
    }


    #[test]
    fn read_until_test() {
        let input = &(string_to_vec(String::from("This\nthat")));
        let context = Context::new(input);

        let result = read_until(context, '\n');

        match result {
            Some((ctx, value)) => {
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


    // Tests for read()
    #[test]
    fn read_terminal() {
        let empty = &(string_to_vec(String::from(EMPTY)));
        let context = Context::new(empty);

        match read(context) {
            Some((_, AbcToken::Terminal)) => assert!(true, "Empty results in Terminal character"),
            _ => assert!(false, "Terminal should be returned"),
        }
    }

}
