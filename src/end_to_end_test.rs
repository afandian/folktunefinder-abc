#[cfg(test)]
use representations;

fn string_to_vec(input: String) -> Vec<char> {
    input.chars().collect::<Vec<char>>()
}

#[test]
fn chromatic() {
    assert_eq!(
        representations::ast_to_pitches(&representations::abc_to_ast(
            &("K:C\nCDEFGABcdefgabc'".to_string())
        )),
        vec![60, 62, 64, 65, 67, 69, 71, 72, 74, 76, 77, 79, 81, 83, 84],
        "C Major scale pitches."
    );

    assert_eq!(
        representations::ast_to_pitches(&representations::abc_to_ast(
            &("K:C\nC ^C D ^D E F ^F G ^G A ^A B c ^c d ^d e f ^f g ^g a ^a b c'".to_string())
        )),
        vec![
            60, 61, 62, 63, 64, 65, 66, 67, 68, 69, 70, 71, 72, 73, 74, 75, 76, 77, 78, 79, 80, 81,
            82, 83, 84,
        ],
        "C chromatic scale pitches using sharps."
    );

    assert_eq!(
        representations::ast_to_pitches(&representations::abc_to_ast(
            &("K:C\nC _D D _E E F _G G _A A _B B c _d d _e e f _g g _a a _b b c'".to_string())
        )),
        vec![
            60, 61, 62, 63, 64, 65, 66, 67, 68, 69, 70, 71, 72, 73, 74, 75, 76, 77, 78, 79, 80, 81,
            82, 83, 84,
        ],
        "C chromatic scale pitches using flats."
    );

    // TODO
    // Pitch resolution for music::PitchClass doesn't respect key or mode!
    // assert_eq!(
    //     representations::ast_to_pitches(&representations::abc_to_ast(
    //         &("K:D\nDEFGABcdefgabc'd'".to_string())
    //     )),
    //     vec![62, 64, 66, 67, 69, 71, 73, 74, 76, 78, 79, 81, 83, 85, 86],
    //     "D scale pitches."
    // );
}
