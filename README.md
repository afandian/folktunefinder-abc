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

## Getting started

Install Cargo.

Install Tarpaulin for test coverage.

  cargo install cargo-tarpaulin

## License

This is open source software, and has the "MIT License", see LICENSE file. 

You are very welcome to use this software in accordance with the license.
If you do, I would be very grateful if you let me know and give credit where appropriate!

