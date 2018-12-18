//! SearchEngine
//! SearchEngine engine that ties various things together.
//! Plus structures for conducting searches and representing results.
//! Results are JSON-serializable.

use std::cmp::Ordering;
use std::collections::HashMap;

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

    pub fn total(&self) -> usize {
        self.results.len()
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Query {
    pub offset: usize,
    pub rows: usize,
    pub melody: Option<Vec<u8>>,
}

// A search engine.
// TODO Trade off storage and pre-parsing of ASTs with RAM usage vs time to fetch / reconstruct data.
// Once we've indexed it we could either keep only the ABC text in memory and parse on demand.
// Or even just store pointers to disk.
pub struct SearchEngine {
    clusters: relations::Clusters,

    // ABCs are shared around threads.
    pub abcs: Arc<storage::ABCCache>,

    pub asts: HashMap<u32, tune_ast_three::Tune>,

    interval_term_vsm: relations::IntervalWindowBinaryVSM,
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
            abcs: abcs_arc,
            interval_term_vsm,
        }
    }

    pub fn search(&self, query: &Query) -> ResultSet {
        // TODO other kinds of searches and filters.

        match query.melody {
            None => ResultSet::new(),
            Some(ref melody) => {
                let search_intervals = representations::pitches_to_intervals(&melody);
                self.interval_term_vsm.search(
                    &search_intervals,
                    0.8,
                    relations::ScoreNormalization::DocA,
                )
            }
        }
    }
}

// A user-facing result with metadata etc.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DecoratedResult {
    pub titles: Vec<String>,
    pub id: usize,
    pub score: f32,
}

pub fn export_results(
    result_set: &ResultSet,
    searcher: &SearchEngine,
    offset: usize,
    rows: usize,
) -> Vec<DecoratedResult> {
    let mut results = vec![];

    for (id, score) in result_set.results.iter() {
        let mut result = DecoratedResult {
            titles: vec![],
            id: *id,
            score: *score,
        };

        results.push(result);
    }

    let total_results = result_set.total();

    // Sort has to be stable, as we're iterating over pages.
    // So sort by ID first.
    results.sort_by_key(|x| x.id);
    results.sort_by(|a, b| a.score.partial_cmp(&b.score).unwrap());

    // TODO - Deduplicate by group ID and roll up into groups, including highest score per group.

    let lower = usize::max(0, usize::min(offset, total_results));
    let upper = usize::max(0, usize::min(offset + rows, total_results));

    results = results[lower..upper].to_vec();

    // Decorate with Titles and maybe other things.
    for result in results.iter_mut() {
        if let Some(ast) = searcher.asts.get(&(result.id as u32)) {
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

    results
}
