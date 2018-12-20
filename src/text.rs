//! Words.

// TODO:
//  - stop words?
//  - synonyms- bourree? jigg?

use std::collections::HashSet;
use unidecode::unidecode;

// Tokenize input a number of different ways.
// These are all unioned.
pub fn tokenize(text: &str) -> HashSet<String> {
    let mut result = HashSet::new();

    // Produce a lower-case, with and without diacritics.
    let lowercase = text.to_lowercase();
    let ascii = unidecode(&lowercase);
    let mut preprocessed = String::new();
    preprocessed.push_str(&lowercase);
    preprocessed.push_str(" ");
    preprocessed.push_str(&ascii);

    // Take a number of steps, adding to the set each time.

    // Always tokenize by whitespace.
    let mut tokenized: Vec<String> = preprocessed
        .split(char::is_whitespace)
        .map(String::from)
        .collect();

    let mut new: Vec<String> = vec![];

    // Tokens that are a mix of alpha and numeric.
    for tok in tokenized.iter() {
        new.extend(tok.split(|x| !char::is_alphanumeric(x)).map(String::from));
    }
    tokenized.append(&mut new);

    // Tokens that are only alpha.
    for tok in tokenized.iter() {
        new.extend(tok.split(|x| !char::is_alphabetic(x)).map(String::from));
    }
    tokenized.append(&mut new);

    // Tokens that are only numeric.
    for tok in tokenized.iter() {
        new.extend(tok.split(|x| !char::is_numeric(x)).map(String::from));
    }
    tokenized.append(&mut new);

    for tok in tokenized.iter() {
        if tok.ends_with("s") {
            // It's safe to do this because 's' is a one-byte character.
            let tok = tok[0..tok.len()].to_string();
            new.push(tok);
        }
    }
    tokenized.append(&mut new);

    for x in tokenized.iter().filter(|x| !String::is_empty(x)) {
        result.insert(x.to_string());
    }

    result
}

#[test]
fn test_regressions() {
    // Test regressions.
    // Real output should be at least a superset of the expected output.
    let tests = vec![
        // Lower case.
        (
            "ONE TWO THREE one two three",
            vec!["one", "two", "three", "one", "two", "three"],
        ),
        // Tokenize alphanumeric strings.
        ("ONE1.TWO2.THREE3?", vec!["one1", "two2", "three3"]),
        // From real inputs.
        (
            "High Part of the Road",
            vec!["high", "part", "of", "the", "road"],
        ),
        ("Reel de Montreal", vec!["reel", "de", "montreal"]),
        ("Sonderhoning 3", vec!["sonderhoning", "3"]),
        (
            "Return From Fingal, The",
            vec!["return", "from", "fingal", "the"],
        ),
        ("Rigaudon de Sveran", vec!["rigaudon", "de", "sveran"]),
        (
            "'Walzer aus Bayern' - Walzer/Waltz/Valse",
            vec!["walzer", "aus", "bayern", "walzer", "waltz", "valse"],
        ),
        (
            "4th Dragoons March. JMT.077",
            vec!["4th", "4", "th", "dragoons", "march", "jmt", "077"],
        ),
        ("A Good Roll-up", vec!["a", "good", "roll", "up"]),
        (
            "A HORNPIPE BY C. SMITH",
            vec!["a", "hornpipe", "by", "c", "smith"],
        ),
        ("A Spanish Jigg.", vec!["a", "spanish", "jigg"]),
        ("AIR XXI. Tweed-side.", vec!["air", "xxi", "tweed", "side"]),
        (
            "AIR XXXVI. Bonniest Lass in all the World.",
            vec![
                "air", "xxxvi", "bonniest", "lass", "in", "all", "the", "world",
            ],
        ),
        (
            "All's Well.Primo. JMT.084",
            vec!["all's", "well", "primo", "jmt.084", "jmt", "084"],
        ),
        (
            "Allonby Lasses. BF13.037",
            vec!["allonby", "lasses", "bf13.037", "bf", "13", "037"],
        ),
        ("Ambrose Moloney's", vec!["ambrose", "moloney"]),
        (
            "Aoife's Come to Dublin",
            vec!["aoife", "come", "to", "dublin"],
        ),
        ("Ash Plant, The", vec!["ash", "plant", "the"]),
        (
            "From Atkins Menagerie. CJF.130",
            vec!["from", "atkins", "menagerie", "cjf.130", "cjf", "130"],
        ),
        (
            "Atkins Menagerie,From. CJF.130",
            vec!["atkins", "menagerie", "from", "cjf.130", "cjf", "130"],
        ),
        (
            "AVANT DEUX de SAINT-GRAVAVANT",
            vec!["avant", "deux", "de", "saint", "gravavant"],
        ),
        ("Kas-a-barh/An Dro", vec!["kas", "a", "barh", "an", "dro"]),
        (
            "Kas a barh (A. Pennec)",
            vec!["kas", "a", "barh", "a", "pennec"],
        ),
    ];

    for (input, expected) in tests {
        let expected: HashSet<String> = expected.iter().map(|x| String::from(*x)).collect();
        let result = tokenize(input);

        let ok = expected.is_subset(&result);
        if !ok {
            eprintln!("Input: {} => {}", input, ok);
            eprintln!("Expected: {:?}", expected);
            eprintln!("  Result: {:?}", result);
            eprintln!(" Missing: {:?}", expected.difference(&result));
            assert!(ok);
        }
    }
}
