# FolkTuneFinder ABC Tools

Tools for working with ABC Notation files (http://abcnotation.com) created whilst making FolkTuneFinder.com.
This is just a bit of fun to try out Rust language. It might go nowhere.

## Aims

 - Well documented, fully tested and generally friendly to a newcomer to the codebase.
 - Parser should be absolutely as friendly as possible, providing hints on error.
 - All useful parts should be available as a Rust library for other people to build on.
 - Parser should be decoupled from utils.
 - Tools should be self-contained in a single repository, with minimal dependencies, and easy to distribute.
 - Successfully parse all valid files in the FolkTuneFinder corpus.

## Code
 - High test coverage.
 - All entities commented.
 - Automatic developerment.
 - No warnings.
 - Formatted with rustfmt for consistency.

## Intended functionality

This will be a general purpose ABC tool. It may provide a range of functionality:

 - Verify and clean ABC files.
 - Rudimentary visualisation via export to SVG.
 - Similarity clustering.
 - MIDI output.

## TODO

 - Handle Windows newline characters.
 - Handle escape sequences for LaTeX accents. 
 - Handle escaped closing square brackets for inline fields.
 - Run over entire folktunefinder.com corpus and make sure all parse errors are well-known (i.e. no UnknownErorrs).

## Potential Features

### Cleanup

 - Uniformalize line endings (based on stats or configurable).
 - Strip and normalize whitespace around headers.
 - Sort headers.
 - Lowest common denomenator in time signature.
 - Shortest possible notation for notes (/ and /2, dotted rhythm).
 - Remove empty text fields.
 - Normalize to Unicode or to escape sequence ASCII.
 - 4/4 => C etc


## To run

Currently work in progress is is 'clean':

    $ cat test_resources/so-far.abc |  target/debug/abctool cleanup

    There were 4 errors!
      | A:AREA
      | B:BOOK
      | C:COMPOSER
      | T:
      | D:DISCOGRAPHY
      | F:FILENAME
      | G:GROUP
      | M:2/
            ^-- ExpectedNumber
      | H:HISTORY
      | I:INFO
      | N:NOTES
      | O:ORIGIN
      | T:
      | S:SOURCE
      | M:
          ^-- PrematureEnd(Metre)
      | T:TITLE
      | M:2/4
      | M:2/4X
              ^-- UnexpectedHeaderLine
      | W:WORDS
      | X:100
      | Z:TRANSCRIPTION
      | M:
          ^-- PrematureEnd(Metre)
      | 


## Getting started

Install Cargo.

Install Tarpaulin for test coverage.

  cargo install cargo-tarpaulin

## License

This is open source software, and has the "MIT License", see LICENSE file. 

You are very welcome to use this software in accordance with the license.
If you do, I would be very grateful if you let me know and give credit where appropriate!

