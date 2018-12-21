# FolkTuneFinder ABC Tools

Tools for working with ABC Notation files (http://abcnotation.com) created whilst making FolkTuneFinder.com.
This is just a bit of fun to try out Rust language. It might go nowhere.

## Current state of play - Dec 2018

If there's anyone watching.

I started this in December 2017, and then had a busy year. As of December 2018:

 - Looks like this will make a viable open source search engine to run FolkTuneFinder.com.
 - Will also remain open to other ABC-related uses. Let me know if you're interested.
 - Scan a directory of ABC tunes, glob into a single file.
 - Cluster tunes by similarity into groups. 
 - Search engine can:
   - Do limited feature extraction (time, key, rhythm)
   - Do limited melody indexing. This is only at proof of concept state just now. (Need to finish ABC parser before doing much more).
   - Do text search for title.
   - Spin up REST API for searching:
     - Exact melody search.
     - Title search.
     - Filter by features (key, time, rhythm).
     - Facet results by features.
     - Pagination.
     - Optional roll-up by tune clusters.
     - Retrieve tune ABC.
   - Very beginnings of a web interface. HTML templates using handlebars, with configurable template directory.


To run:

    HTTP_BIND=0.0.0.0:8765 RUST_BACKTRACE=1 BASE=~/personal/tune-db cargo run --release server
    
Then visit:

    http://localhost:8765/api/v3/tunes/search?interval_ngram=60,62,64,65,67,69&rollup=false&facet=true&key=G

Using configurable template directory (WIP):

    HTML_TEMPLATES=demo-html-templates HTTP_BIND=0.0.0.0:8765 RUST_BACKTRACE=1 BASE=~/personal/tune-db cargo run --release server

then:

    http://localhost:8765/tunes?interval_ngram=60,62,64,65,67,69&rollup=false&facet=true&key=G


Search params:

 - Search:
    - `interval_ngram` - Supply a sequence of pitches, search by ngram.
    - `title` - Supply some title text, search by that.
    - If neither is supplied, return all tunes.
 - Filter:
    - `metre`, e.g. `metre=4/4`
    - `key`, e.g. `key=A'
    - `key-signature` e.g. `key-signature=A-Dorian`
    - `metre-beats`, e.g. `metre-beats=4`
    - `mode`, e.g. `mode=Major`
    - `rhythm`, e.g. `rhythm=jig`. NB this is currently index un-normalized as supplied in the ABC and mostly useless.
    - For a full set of filter types and values, visit `/api/v3/features` or look in the facets of search results.
 - Selection:
    - `rows` - page size, e.g. `rows=20`
    - `offset` - page starting point, e.g. `offset=20`
    - `facet` - Include facets? This gives a breakdown of feature types and values, along with counts, that can be used to further filter. e.g. `facet=true`
    - `include-abc` - Not yet implemented.
    - `rollup` - Roll up duplicates (i.e. so similar as to be transcriptions of the same thing) so that only the best match from each tune is shown. The total number of results is shown in the results, along with the number of 'unique' results.
 
Room for improvement:

 - Has an ABC lexer, parser, AST. However it's not totally finished and will require more tweaking, test cases.
 - Typesetting is probably a dead end. There are other packages that do this.
 - Feature extraction can be improved. Probably create a vocabulary of rythm types.

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
 - `DEBUG_MAX_ID` - limit tune top id to this value. Selects a subset for profiling, debugging, etc.

## Performance

On a random Macbook air, full scan of 200,000 tunes and error reporting:

 - DEBUG: 3m5.416s
 - RELEASE: 0m47.434s = ~4x speedup

## Getting started

Install Cargo.

Install Tarpaulin for test coverage.

  cargo install cargo-tarpaulin

## License

This is open source software, and has the "MIT License", see LICENSE file. 

You are very welcome to use this software in accordance with the license.
If you do, I would be very grateful if you let me know and give credit where appropriate!

