//! Tune Abstract Syntax Tree
//! Turns an ABC token stream into a more useful structure.

use abc_lexer as l;


/// Vocabulary of object types.
/// These are similar but different to the various lexer tokens.
enum HeaderField {
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
    Metre(u32, u32),
}

pub struct TuneAst {
    headers: Vec<HeaderField>,
    errors: Vec<(usize, l::LexError)>,
}

impl TuneAst {
    pub fn new() -> TuneAst {
        TuneAst {
            headers: vec![],
            errors: vec![],
        }
    }

    fn add_header(&mut self, header_field: HeaderField) {
        self.headers.push(header_field);
    }

    fn add_error(&mut self, index: usize, error: l::LexError) {
        self.errors.push((index, error));
    }

    pub fn num_errors(&self) -> usize {
        self.errors.len()
    }
}

/// Read from a Lexer and build a new AST.
pub fn read_from_lexer(lexer: l::Lexer, ast: &mut TuneAst) {
    for token in lexer {
        match token {
            // On error extract the index from the context. That's the only bit we need.
            // Keeping the context confers the lifetime of the underlying ABC char slice on the AST.
            // Coupling the AST to its source isn't desirable. The index is all we need to store.
            // Using it with the input to print errors can exist in a parent context.
            l::LexResult::Error(_, offset, error) => ast.add_error(offset, error),

            // If there's a token we don't care about the context.
            l::LexResult::T(_, token) => {
                match token {

                    l::T::Terminal => (),
                    // TODO depending on tune section this may mean start a new line of music.
                    l::T::Newline => (),
                    l::T::Area(value) => ast.add_header(HeaderField::Area(value)),
                    l::T::Book(value) => ast.add_header(HeaderField::Book(value)),
                    l::T::Composer(value) => ast.add_header(HeaderField::Composer(value)),
                    l::T::Discography(value) => ast.add_header(HeaderField::Discography(value)),
                    l::T::Filename(value) => ast.add_header(HeaderField::Filename(value)),
                    l::T::Group(value) => ast.add_header(HeaderField::Group(value)),
                    l::T::History(value) => ast.add_header(HeaderField::History(value)),
                    l::T::Information(value) => ast.add_header(HeaderField::Information(value)),
                    l::T::Notes(value) => ast.add_header(HeaderField::Notes(value)),
                    l::T::Origin(value) => ast.add_header(HeaderField::Origin(value)),
                    l::T::Source(value) => ast.add_header(HeaderField::Source(value)),
                    l::T::Title(value) => ast.add_header(HeaderField::Title(value)),
                    l::T::Words(value) => ast.add_header(HeaderField::Words(value)),
                    l::T::X(value) => ast.add_header(HeaderField::X(value)),
                    l::T::Transcription(value) => ast.add_header(HeaderField::Transcription(value)),
                    l::T::Metre(numerator, denomenator) => {
                        ast.add_header(HeaderField::Metre(numerator, denomenator))
                    }
                }
            }

        }
    }
}
