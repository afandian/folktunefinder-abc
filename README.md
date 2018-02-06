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


## Error checking

        $ cat test_resources/so-far.abc |  target/debug/abctool check

    There was 1 error!
      | M:
      >   ^-- I've got to the end of the ABC tune before I'm ready.
              I was in the middle of reading a time signature
      |

    There was 1 error!
      | M:3
      >    ^-- I expected to find a slash for the time signature.
      |

    There was 1 error!
      | M:3/
      >     ^-- I expected to find a number here.
      |

    There was 1 error!
      | M:3/4
      |
      > ^-- I expected to find a header, but found something else.
      |

    There were 2 errors!
      | M:23456789012/1234567890
      >   ^-- This number is longer than I expected.
      | T:Hello
      | M:1111
      >       ^-- I expected to find a slash for the time signature.
      | T:This
      |


    There were 2 errors!
      | M:23456789012/1234567890
      >   ^-- This number is longer than I expected.
      | T:Hello
      | M:1111
      >       ^-- I expected to find a slash for the time signature.
      | T:This
      |


    There was 1 error!
      | M:
      >   ^-- I've got to the end of the ABC tune before I'm ready.
              I was in the middle of reading a time signature


## Typesetting

Mega work-in-progress.

    $ cat test_resources/butterfly.abc |  target/debug/abctool typeset

With minims and crotchets:

<img src="progress/2018-01-29 at 23.07.52.png">

## Scan Tune Database

A database of ABC tunes is stored in a cache. They are read from the filesystem in the directory specified by the `BASE` evironment variable. Files can be anywhere in the directory hierarchy, but should each have distinct numerical names, such as `1001.abc`. 

To update the tune database:

    BASE=/path/to/abcs cargo run scan

The tunecache file will be stored at `/path/to/abcs/tunecache`. When new tunes are added, run re-scan. Only new files will be added. It is a simple concatenation of the files into one blob, with tune IDs and length data. Because reading hundreds of thousands of files is slow, database-oriented functions work from this cache.

## Run server

Serve up ABC, typeset SVG, and in future, perform search:

    HTTP_BIND=0.0.0.0:3000 BASE=~/tune-db cargo run server

## Config

 - `BASE` - where are the ABC tunes? e.g. /tmp/tunes
 - `HTTP_BIND` - http bind address and port for server? e.g. 0.0.0.0:8000

## Getting started

Install Cargo.

Install Tarpaulin for test coverage.

  cargo install cargo-tarpaulin

## License

This is open source software, and has the "MIT License", see LICENSE file. 

You are very welcome to use this software in accordance with the license.
If you do, I would be very grateful if you let me know and give credit where appropriate!

