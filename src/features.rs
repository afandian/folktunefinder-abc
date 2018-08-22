use abc_lexer as l;
use music;
use tune_ast_three;

pub fn key_signature(ast: &tune_ast_three::Tune, result: &mut Vec<(String, String)>) {
    // TODO if there's no Key, assume C major.
    for ref token in ast.prelude.iter() {
        match *token {
            l::T::KeySignature(pitch_class, mode) => {
                result.push(("key".to_string(), pitch_class.to_string()));
                result.push(("mode".to_string(), mode.to_string()));
                result.push((
                    "key-signature".to_string(),
                    format!("{}-{}", pitch_class.to_string(), mode.to_string()),
                ));
            }
            _ => (),
        }
    }

    for ref voice in ast.voices.iter() {
        for ref token in voice.iter() {
            match *token {
                l::T::KeySignature(pitch_class, mode) => {
                    result.push(("key".to_string(), pitch_class.to_string()));
                    result.push(("mode".to_string(), mode.to_string()));
                    result.push((
                        "key-signature".to_string(),
                        format!("{}-{}", pitch_class.to_string(), mode.to_string()),
                    ));
                }
                _ => (),
            }
        }
    }
}

pub fn time_signature(ast: &tune_ast_three::Tune, result: &mut Vec<(String, String)>) {
    // TODO if there's no entry, assume 4/4.
    for ref token in ast.prelude.iter() {
        match *token {
            l::T::Metre(metre) => {
                let music::Metre(numerator, _) = metre;
                result.push(("metre".to_string(), metre.to_string()));
                result.push(("metre-beats".to_string(), numerator.to_string()));
            }
            _ => (),
        }
    }

    for ref voice in ast.voices.iter() {
        for ref token in voice.iter() {
            match *token {
                l::T::Metre(metre) => {
                    let music::Metre(numerator, _) = metre;
                    result.push(("metre".to_string(), metre.to_string()));
                    result.push(("metre-beats".to_string(), numerator.to_string()));
                }
                _ => (),
            }
        }
    }
}

pub fn rhythm(ast: &tune_ast_three::Tune, result: &mut Vec<(String, String)>) {
    // TODO if there's no entry, assume 4/4.
    for ref token in ast.prelude.iter() {
        match *token {
            // TODO normalize
            l::T::Rhythm(value) => {
                result.push(("rhythm".to_string(), value.to_string()));
            }
            _ => (),
        }
    }

    for ref voice in ast.voices.iter() {
        for ref token in voice.iter() {
            match *token {
                l::T::Rhythm(value) => {
                    result.push(("rhythm".to_string(), value.to_string()));
                }
                _ => (),
            }
        }
    }
}

//
pub fn extract_all_features(ast: &tune_ast_three::Tune) -> Vec<(String, String)> {
    let mut result = vec![];

    key_signature(ast, &mut result);
    time_signature(ast, &mut result);
    rhythm(ast, &mut result);

    result
}
