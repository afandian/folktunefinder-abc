## Goals

 - Parse all of ABC, eventually.
 - Be able to skip bits we don't understand.
 - All features in AST should render back to ABC.
 - Only need to be able to represent that subset of musical structures that can be represented in ABC.
 - AST data structure should Round-trip via ABC.


## Decisions

 - Hierarchical bar structure means that we can't have beam groups spanning a bar line.
 - Hierarchical bar structure means that we can't have n-let groups spanning a bar line.

## ABC Features

|Feature|Lex|AST|AST->ABC|Typeset|
|-------|---|---|--------|-------|
| Durations with multiple slashes. | | | |
| Polyphony: Multi-voice bars. | | | |
| Polyphony: Multi-voice systems. | | | |
| Polyphony: Multi-pitch notes. | | | |
| Guitar chords | | | |
| Dotted durations using ">" and more. | | | |
| Repeat bars. | | | |
| Ornaments. | | | |
| LaTeX accents. | | | |
| Basic bars. | X | | |
| Textual headers. | X | X | |
| Notes with full pitch. | X | X | |
| Default note length | X | N | N | N |
| Mid-tune change default note length | | | | |
| Mid-tune whole-line header fields | | | | 
| Mid-tune bracketed header fields | | | |
| Mid-tune multi-bracketed header fields | | | |
| Rests with "z" | | | |
| Empty with "x"
| Accidentals | X | X | | |
| Key signature affects note pitch | | | |
| Key signature header | X | X | | |
| Highland pipe mode | | | |
| Extra accidental in key signature | | | |
| Mid-tune key signature | | | |
| Time signature | X | X | | 
| Mid-tune time signature | | | | |
| Multiple tunes per input file | | | |
| Tempo field | | | |
| Ornaments | | | |
| Grace notes in braces | | | |
| Ties, incl over barline | | | |
| Slurs, incl over barline | | | |
| Nested slurs | | | |
| n-lets | | | |
| Guitar chords in quotes and + | | | |
| End-of-line continuation with "\\" | | | |
| Force end of line with "!" | | | |
| Up and downbow | | | |
| Accents with "." | | | |
| Parts | | | |
| Comments | | | |
| Extra accents e.g. T | | | |
| Accents like "!fermata!" | | | |
| Annotation in guitar chords | | | |
| Song word alignment | | | |
| Voices and e.g. clefs  | | | |



