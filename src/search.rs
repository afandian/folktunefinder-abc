//! SearchEngine
//! SearchEngine engine that ties various things together.
//! Plus structures for conducting searches and representing results.
//! Results are JSON-serializable.
//!
//! Query syntax, presented as key-value from query string:
//! Filters:
//!  - todo
//!  -
//!
//! Generators:
//!  - all
//!  - interval_ngram
//!  - degree_ngram
//!  - interval_histogram
//!  - degree_histogram
//!
//! Select:
//!  - offset
//!  - rows
//!  - include_abc
//!  - include_proxy
//!  -

use std::cmp::Ordering;
use std::collections::HashMap;
use std::collections::HashSet;

use abc_lexer as l;
use relations;
use representations;
use storage;
use tune_ast_three;

use std::sync::Arc;

use serde;
use serde_derive;
use serde_json;

// Simple lightweight tune ID to weight for collecting results.
#[derive(Debug)]
pub struct ResultSet {
    // Tune id => weight.
    results: HashMap<usize, f32>,
}

impl ResultSet {
    pub fn new() -> ResultSet {
        ResultSet {
            results: HashMap::new(),
        }
    }

    pub fn add(&mut self, tune_id: usize, score: f32) {
        self.results.insert(tune_id, score);
    }

    // Return a sorted vec of (tune id, score).
    // TODO This method is unused.
    pub fn results(&self) -> Vec<(u32, f32)> {
        let mut result = Vec::<(u32, f32)>::new();

        for (id, score) in self.results.iter() {
            result.push((*id as u32, *score));
        }

        // Sort descending by score.
        result.sort_by(|(a_id, a_score), (b_id, b_score)| {
            b_score.partial_cmp(a_score).unwrap_or(Ordering::Equal)
        });

        result
    }

    // TODO misleading name!
    pub fn total(&self) -> usize {
        self.results.len()
    }
}

// A Generator supplies a weighted result set. Only one generator per result.
#[derive(Debug, Serialize, Deserialize)]
pub enum Generator {
    // All tunes, weighted by ID.
    All,

    // Search by interval n-gram similarity, weighted by similarity.
    IntervalNGram(Vec<u8>),

    // Search by degree n-gram similarity, weighted by similarity.
    DegreeNGram(Vec<u8>),

    // Search by interval histogram similarity, weighted by similarity.
    IntervalHistogram(Vec<f32>),

    // Search by degree histogram similarity, weighted by similarity.
    DegreeHistogram(Vec<f32>),
}

// A filter selects items in the result set.
// All terms are ANDed.
#[derive(Debug, Serialize, Deserialize)]
pub struct Filter {
    // TODO
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Selection {
    // Start at this index of the results.
    pub offset: usize,

    // Return only this many rows.
    pub rows: usize,

    // Roll-up tunes based on their cluster.
    // When true, return only the best tune per group.
    pub rollup: bool,

    // Include the ABC text.
    pub include_abc: bool,

    // Include the proxy object.
    pub include_proxy: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Query {
    pub generator: Generator,
    pub filter: Filter,
    pub selection: Selection,
}

fn parse_filter(params: &HashMap<String, String>) -> Result<Filter, String> {
    // TODO we don't have any yet.
    Ok(Filter {})
}

const DEFAULT_ROWS: usize = 30;
const MAX_ROWS: usize = 1000;

// One octave above and below key note.
const HISTOGRAM_LENGTH: usize = 25;

fn parse_selection(params: &HashMap<String, String>) -> Result<Selection, String> {
    let offset: usize = match params.get("offset") {
        Some(v) => match v.parse::<usize>() {
            Ok(v) => v,
            Err(_) => return Err("Invalid value for 'offset'.".to_string()),
        },
        _ => 0,
    };

    let rows: usize = match params.get("rows") {
        Some(v) => match v.parse::<usize>() {
            Ok(v) if (v <= MAX_ROWS) => v,
            Ok(v) => return Err("Too many rows requested".to_string()),
            Err(_) => return Err("Invalid value for 'rows'".to_string()),
        },
        _ => DEFAULT_ROWS,
    };

    let rollup = match params.get("rollup") {
        Some(val) => match val.as_ref() {
            "true" => true,
            "false" => false,
            _ => return Err("Invalid value for 'rollup'".to_string()),
        },
        _ => false,
    };

    let include_abc = match params.get("include_abc") {
        Some(val) => match val.as_ref() {
            "true" => true,
            "false" => false,
            _ => return Err("Invalid value for 'include_abc'".to_string()),
        },
        _ => false,
    };

    let include_proxy = match params.get("include_proxy") {
        Some(val) => match val.as_ref() {
            "true" => true,
            "false" => false,
            _ => return Err("Invalid value for 'include_proxy'".to_string()),
        },
        _ => false,
    };

    Ok(Selection {
        offset,
        rows,
        rollup,
        include_abc,
        include_proxy,
    })
}

fn parse_generator(params: &HashMap<String, String>) -> Result<Generator, String> {
    // This argument is given as absolute pitches, at least for now.
    // Would be more consistent to convert it to intervals prior to querying API perhaps...
    if let Some(val) = params.get("interval_ngram") {
        match val.split(",").map(|s| s.parse::<u8>()).collect() {
            Ok(value) => return Ok(Generator::IntervalNGram(value)),
            Err(_) => return Err("Invalid value given for 'interval_ngram'".to_string()),
        }
    }

    if let Some(val) = params.get("degree_ngram") {
        match val.split(",").map(|s| s.parse::<u8>()).collect() {
            Ok(value) => return Ok(Generator::DegreeNGram(value)),
            Err(_) => return Err("Invalid value given for 'degree_ngram'".to_string()),
        }
    }

    if let Some(val) = params.get("interval_histogram") {
        let result: Result<Vec<_>, _> = val.split(",").map(|s| s.parse::<f32>()).collect();
        match result {
            Ok(value) => {
                if (value.len() == HISTOGRAM_LENGTH) {
                    return Ok(Generator::IntervalHistogram(value));
                } else {
                    return Err(format!(
                        "Invalid length for 'interval_histogram'. Must be exactly {}",
                        HISTOGRAM_LENGTH
                    ));
                }
            }
            Err(_) => return Err("Invalid value given for 'interval_histogram'".to_string()),
        }
    }

    if let Some(val) = params.get("degree_histogram") {
        let result: Result<Vec<_>, _> = val.split(",").map(|s| s.parse::<f32>()).collect();
        match result {
            Ok(value) => if (value.len() == HISTOGRAM_LENGTH) {
                return Ok(Generator::DegreeHistogram(value));
            } else {
                return Err(format!(
                    "Invalid length for 'interval_histogram'. Must be exactly {}",
                    HISTOGRAM_LENGTH
                ));
            },
            Err(_) => return Err("Invalid value given for 'interval_histogram'".to_string()),
        }
    }

    Ok(Generator::All)
}

pub fn parse_query(params: &HashMap<String, String>) -> Result<Query, String> {
    eprintln!("Search query: {:?}", params);

    let filter = match parse_filter(params) {
        Ok(filter) => filter,
        Err(message) => return Err(message),
    };

    let selection = match parse_selection(params) {
        Ok(selection) => selection,
        Err(message) => return Err(message),
    };

    let generator = match parse_generator(params) {
        Ok(generator) => generator,
        Err(message) => return Err(message),
    };

    Ok(Query {
        filter,
        selection,
        generator,
    })
}

// A search engine.
// TODO Trade off storage and pre-parsing of ASTs with RAM usage vs time to fetch / reconstruct data.
// Once we've indexed it we could either keep only the ABC text in memory and parse on demand.
// Or even just store pointers to disk.
pub struct SearchEngine {
    // Clusters of 'the same' tune.
    // Used for optional (default) roll-up to dedupe very similar or identical results.
    clusters: relations::Clusters,

    // ABCs are shared around threads.
    // TODO We may not need to put this in ARC.
    pub abcs: Arc<storage::ABCCache>,

    // Parsed ASTs.
    // TODO do we need to retain this?
    pub asts: HashMap<u32, tune_ast_three::Tune>,

    // Tune features in a binary VSM.
    features: relations::FeaturesBinaryVSM,

    // Interval window VSM for melody searching.
    // TODO normalize this to the other nomenclature 0f interval / degree + histogram / ngram.
    interval_term_vsm: relations::IntervalWindowBinaryVSM,
    //  TODO Text VSM.
}

impl SearchEngine {
    pub fn new(abcs: storage::ABCCache, clusters: relations::Clusters) -> SearchEngine {
        let abcs_arc = Arc::new(abcs);

        eprintln!("Parsing ABCs...");
        let asts = representations::abc_to_ast_s(&abcs_arc);

        eprintln!("Indexing melody...");
        let pitches = representations::ast_to_pitches_s(&asts);
        let intervals = representations::pitches_to_intervals_s(&pitches);
        let interval_term_vsm = representations::intervals_to_binary_vsm(&intervals);

        eprintln!("Building feature index...");
        let features = representations::asts_to_features_s(&asts);

        // TODO allow filtering by features, search by intervals.
        // TODO build text index.
        // TODO build synonyms and development tools for features, specifically Rhythm.

        SearchEngine {
            clusters,
            asts,
            features,
            abcs: abcs_arc,
            interval_term_vsm,
        }
    }

    pub fn search(
        &self,
        query: &Query,
    ) -> (
        // Total results.
        usize,
        // Result set (may be cut down by dedupe).
        usize,
        Vec<DecoratedResult>,
    ) {
        // First generate a weighted set.

        let generated = match query.generator {
            // TODO should be all
            Generator::All => {
                let mut results = ResultSet::new();
                for i in 0..self.abcs.max_id() {
                    results.add(i as usize, 1.0);
                }
                results
            }
            Generator::IntervalNGram(ref melody) => {
                let search_intervals = representations::pitches_to_intervals(&melody);
                self.interval_term_vsm.search(
                    &search_intervals,
                    0.8,
                    relations::ScoreNormalization::DocA,
                )
            }

            // TODO implement other generators.
            _ => ResultSet::new(),
        };

        // Then generate a filter set.
        let filtered = generated;
        // TODO

        // Then do selection.

        let mut results: Vec<DecoratedResult> = vec![];
        for (id, score) in filtered.results.iter() {
            let mut result = DecoratedResult {
                titles: vec![],
                id: *id,
                score: *score,
            };

            results.push(result);
        }

        let total_results = results.len();

        // Sort has to be stable, as we're iterating over pages.
        // So sort by ID first.
        results.sort_by_key(|x| x.id);
        results.sort_by(|a, b| a.score.partial_cmp(&b.score).unwrap());

        // If this is set (and it is by default) only include the first (best) result in any group.
        let mut results: Vec<DecoratedResult> = if query.selection.rollup {
            let mut seen = HashSet::new();
            let mut new_results: Vec<DecoratedResult> = vec![];

            // Results are sorted best-first, so the first result in any group should stay,
            // the rest should go.
            for result in results.drain(..) {
                match self.clusters.get(result.id) {
                    // If it's not in a group, add as normal.
                    None => new_results.push(result),

                    Some(group_id) => if !seen.contains(&group_id) {
                        seen.insert(group_id);
                        new_results.push(result);
                    },
                }
            }
            new_results
        } else {
            results
        };

        // The number of results from the set we're going to return.
        let num_unique_results = results.len();

        let lower = usize::max(0, usize::min(query.selection.offset, num_unique_results));
        let upper = usize::max(
            0,
            usize::min(
                query.selection.offset + query.selection.rows,
                num_unique_results,
            ),
        );

        results = results[lower..upper].to_vec();

        // Decorate with Titles and maybe other things.
        for result in results.iter_mut() {
            if let Some(ast) = self.asts.get(&(result.id as u32)) {
                let titles = ast
                    .prelude
                    .iter()
                    .filter_map(|x| match x {
                        l::T::Title(x) => Some((*x).clone()),
                        _ => None,
                    }).collect();
                result.titles = titles;
            }
        }

        (total_results, num_unique_results, results)
    }
}

// A user-facing result with metadata etc.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DecoratedResult {
    pub titles: Vec<String>,
    pub id: usize,
    pub score: f32,
}
