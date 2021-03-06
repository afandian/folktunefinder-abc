//! SearchEngine
//! SearchEngine engine that ties various things together.
//! Plus structures for conducting searches and representing results.
//! Results are JSON-serializable.
//!
//! Query syntax, presented as key-value from query string:
//! Filters:
//!  - these depend on the data
//!
//! Generators:
//!  - all
//!  - interval_ngram
//!  - title
//!  - degree_ngram
//!  - interval_histogram
//!  - degree_histogram
//!
//! Select:
//!  - offset
//!  - rows
//!  - rollup

use std::cmp::Ordering;
use std::collections::hash_map::Entry;
use std::collections::HashMap;
use std::collections::HashSet;
use std::path::PathBuf;

use abc_lexer as l;
use pitch;
use relations;
use representations;
use storage;
use tune_ast_three;

use std::sync::Arc;

// We think there will be about this many text terms.
// The load factor of the VSM with real data should dermine this.
// Tweak until the balance is right.
const TEXT_SIZE: usize = 65432;

// We think there will be about this many features.
// The number of features is small and in theory bounded.
// We want matchines to be exact with no collisions.
const FEATURES_SIZE: usize = 512;

const INTERVAL_TERM_SIZE: usize = 16127;

// Simple lightweight tune ID to weight for collecting results.
#[derive(Debug)]
pub struct ResultSet {
    // Tune id => weight.
    pub results: HashMap<usize, f32>,
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
        result.sort_by(|(_a_id, a_score), (_b_id, b_score)| {
            b_score.partial_cmp(a_score).unwrap_or(Ordering::Equal)
        });

        result
    }

    // TODO misleading name!
    pub fn total(&self) -> usize {
        self.results.len()
    }

    // Filter results in this set by intersecting with the supplied filter.
    pub fn filter_by(&mut self, filter_set: &ResultSet) {
        self.results
            .retain(|&id, _| filter_set.results.contains_key(&id));
    }
}

// A Generator supplies a weighted result set. Only one generator per result.
#[derive(Debug, Serialize, Deserialize)]
pub enum Generator {
    // All tunes, weighted by ID.
    All,

    Title(String),

    // Search by interval n-gram similarity, weighted by similarity.
    IntervalNGram(Vec<u8>),

    // Search by degree n-gram similarity, weighted by similarity.
    // TODO not yet implemented.
    DegreeNGram(Vec<u8>),

    // Search by interval histogram similarity, weighted by similarity.
    // TODO not yet implemented.
    IntervalHistogram(Vec<f32>),

    // Search by degree histogram similarity, weighted by similarity.
    // TODO not yet implemented.
    DegreeHistogram(Vec<f32>),
}

// A filter selects items in the result set.
// All terms are ANDed.
#[derive(Debug, Serialize, Deserialize)]
pub struct Filter {
    pub features: Vec<(String, String)>,
}

impl Filter {
    pub fn has_filters(&self) -> bool {
        self.features.len() > 0
    }
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

    // Include facets for all features.
    pub facet: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Query {
    pub generator: Generator,
    pub filter: Filter,
    pub selection: Selection,
}

const DEFAULT_ROWS: usize = 30;
const MAX_ROWS: usize = 1000;

// One octave above and below key note.
const HISTOGRAM_LENGTH: usize = 25;

// Options for which features to enable in the search engine. We don't always want all of them.
pub struct SearchEngineFeatures {
    pub index_text: bool,
    pub index_melody_interval_term: bool,
    pub index_features: bool,
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
    pub abc_cache: storage::ReadOnlyCache,

    // Tune features in a binary VSM.
    pub features_vsm: relations::FeaturesBinaryVSM,

    // Interval window VSM for melody searching.
    // TODO normalize this to the other nomenclature 0f interval / degree + histogram / ngram.
    pub interval_term_vsm: relations::IntervalWindowBinaryVSM,

    // Index of title text.
    pub text_vsm: relations::TextVSM,

    // Cache of all known features.
    all_features_cached: HashMap<String, Vec<String>>,

    max_tune_id: u32,
}

impl SearchEngine {
    pub fn new(
        cache_path: PathBuf,
        clusters: relations::Clusters,
        features: SearchEngineFeatures,
    ) -> SearchEngine {
        // TODO build synonyms and development tools for features, specifically Rhythm.

        let scanner = storage::CacheScanner::new(cache_path.clone());
        let max_tune_id = scanner.iter().map(|x| x.tune_id).max().unwrap_or(0);

        // Melodic index.
        let mut interval_term_vsm =
            relations::IntervalWindowBinaryVSM::new(INTERVAL_TERM_SIZE, max_tune_id as usize);

        // Feature index.
        let mut features_vsm =
            relations::FeaturesBinaryVSM::new(FEATURES_SIZE, max_tune_id as usize);

        // Title text index.
        let mut text_vsm = relations::TextVSM::new(TEXT_SIZE, max_tune_id as usize);

        for (cnt, entry) in scanner.iter().enumerate() {
            if (cnt % 1000) == 0 {
                eprintln!("Indexing {}...", cnt);
            }
            let ast = representations::abc_to_ast(&entry.content);

            // Extract features, insert into VSM.
            if features.index_features {
                let features = representations::ast_to_features(&ast);
                for (feature_type, feature_value) in features {
                    features_vsm.add(entry.tune_id as usize, feature_type, feature_value);
                }
            }

            // Extract title text, insert into VSM.
            if features.index_text {
                let titles = ast.prelude.iter().filter_map(|x| match x {
                    l::T::Title(x) => Some((*x).clone()),
                    _ => None,
                });

                for title in titles {
                    text_vsm.add(entry.tune_id as usize, title);
                }
            }

            // Melodic index.
            if features.index_melody_interval_term {
                let pitches = pitch::PitchSequence::from_ast(&ast);
                let intervals = pitch::IntervalSequence::from_pitch_sequence(&pitches);
                interval_term_vsm.add(entry.tune_id as usize, &intervals.intervals);
            }
        }
        eprintln!("Indexed all tunes.");

        let (distinct_terms, vector_width, load_factor) = text_vsm.vsm.load_factor();
        eprintln!(
            "Text: distinct_terms: {}, vector_width: {}, load_factor: {})",
            distinct_terms, vector_width, load_factor
        );

        // Keep a copy of all known features.
        let all_features_cached = features_vsm.all_features();

        // Now build a cache for future access to ABCs.
        eprintln!("Building file offset index...");
        let abc_cache = storage::ReadOnlyCache::new(cache_path).unwrap();

        eprintln!("Done!");
        SearchEngine {
            clusters,
            features_vsm,
            text_vsm,
            all_features_cached,
            abc_cache,
            interval_term_vsm,
            max_tune_id,
        }
    }

    fn parse_filter(&self, params: &Vec<(String, String)>) -> Result<Filter, String> {
        // The syntax depends on the features we've extracted from the corpus. Whilst the set of
        // feature types is hard-coded, it's best to make the parsing data-driven. This couples the
        // search to the present corpus not the code.

        // Filter and take a copy of those filter key value pairs that correspond to known features.
        let relevant: Vec<(String, String)> = params
            .iter()
            .filter_map(|(k, v)| {
                if self.all_features_cached.contains_key(k) {
                    Some((k.to_string(), v.to_string()))
                } else {
                    None
                }
            }).collect();

        Ok(Filter { features: relevant })
    }

    fn parse_bool(
        &self,
        params: &HashMap<String, String>,
        param_name: &str,
        default: bool,
    ) -> Result<bool, String> {
        match params.get(param_name) {
            Some(val) => match val.as_ref() {
                // HTML forms use on/off . API usage is more conventional true/false.
                "true" | "on" => Ok(true),
                "false" | "off" => Ok(false),
                _ => Err(format!("Invalid value for '{}'", &param_name).to_string()),
            },
            _ => Ok(default),
        }
    }

    fn parse_selection(&self, params: &HashMap<String, String>) -> Result<Selection, String> {
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
                Ok(_v) => return Err("Too many rows requested".to_string()),
                Err(_) => return Err("Invalid value for 'rows'".to_string()),
            },
            _ => DEFAULT_ROWS,
        };

        let rollup = match self.parse_bool(&params, "rollup", true) {
            Ok(val) => val,
            Err(x) => return Err(x),
        };

        let facet = match self.parse_bool(&params, "facet", true) {
            Ok(val) => val,
            Err(x) => return Err(x),
        };

        Ok(Selection {
            offset,
            rows,
            rollup,
            facet,
        })
    }

    fn parse_generator(&self, params: &HashMap<String, String>) -> Result<Generator, String> {
        // This argument is given as absolute pitches, at least for now.
        // Would be more consistent to convert it to intervals prior to querying API perhaps...

        match params.get("title") {
            Some(val) if val.len() > 0 => return Ok(Generator::Title(val.to_string())),
            _ => (),
        }

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
                    if value.len() == HISTOGRAM_LENGTH {
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
                Ok(value) => if value.len() == HISTOGRAM_LENGTH {
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

    pub fn parse_query(&self, params: Vec<(String, String)>) -> Result<Query, String> {
        eprintln!("Search query: {:?}", &params);

        let params_map: HashMap<_, _> = params.clone().into_iter().collect();

        let filter = match self.parse_filter(&params) {
            Ok(filter) => filter,
            Err(message) => return Err(message),
        };

        let selection = match self.parse_selection(&params_map) {
            Ok(selection) => selection,
            Err(message) => return Err(message),
        };

        let generator = match self.parse_generator(&params_map) {
            Ok(generator) => generator,
            Err(message) => return Err(message),
        };

        Ok(Query {
            filter,
            selection,
            generator,
        })
    }

    pub fn search(
        &mut self,
        query: &Query,
    ) -> (
        // Total results.
        usize,
        // Result set (may be cut down by dedupe).
        usize,
        // Facet for (whole) result set.
        Option<HashMap<String, Vec<(String, u32)>>>,
        // Page of results.
        Vec<DecoratedResult>,
    ) {
        // First generate a weighted set.

        let mut generated = match query.generator {
            // TODO should be all
            Generator::All => {
                let mut results = ResultSet::new();
                for i in 0..self.abc_cache.max_id() {
                    results.add(i as usize, 1.0);
                }
                results
            }
            Generator::IntervalNGram(ref melody) => {
                let search_pitches = pitch::PitchSequence::from_pitches(melody);
                let search_intervals =
                    pitch::IntervalSequence::from_pitch_sequence(&search_pitches);
                self.interval_term_vsm.search(
                    &search_intervals.intervals,
                    0.8,
                    relations::ScoreNormalization::DocA,
                )
            }
            Generator::Title(ref text) => self.text_vsm.search(text.to_string()),

            // TODO implement other generators.
            _ => ResultSet::new(),
        };

        // Then generate a filter set. This is all docs that match the filter.
        // TODO it may be more efficient to build this as a predicate that can be supplied to the
        // generators. Really depensd on the balance of usage, and whether generator or filter sets
        // are larger.
        let filtered_results = self.generate_filter_resultset(query);

        // Apply filter to the generated result set.
        // If no filters were supplied, don't perform the filter.
        // This is important because we need to tell the difference between an empty set because
        // there were no matches vs an empty set becuase there was no filter.
        if query.filter.has_filters() {
            generated.filter_by(&filtered_results);
        };

        // Then generate facets if they were requested.
        let facets = if query.selection.facet {
            Some(self.features_vsm.facet_features_for_resultset(&generated))
        } else {
            None
        };

        // Then do selection.
        let mut results: Vec<DecoratedResult> = vec![];
        for (id, score) in generated.results.iter() {
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
        results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap());

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
        // TODO Store metadata a bit better. This involves jumping all over the file currently.
        for result in results.iter_mut() {
            if let Some(entry) = self.abc_cache.get(result.id as u32) {
                let ast = representations::abc_to_ast(&entry);
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

        (total_results, num_unique_results, facets, results)
    }

    // Produce a result set by applying filters.
    // These are ORed within a type, then ANDed.
    fn generate_filter_resultset(&self, query: &Query) -> ResultSet {
        // Into type -> [(type, val)...]
        let mut groups: HashMap<String, Vec<(String, String)>> = HashMap::new();
        for (typ, val) in query.filter.features.iter() {
            // We still want to store these as tuples for later use.
            let tuple = (typ.to_string(), val.to_string());
            match groups.entry(typ.to_string()) {
                Entry::Occupied(o) => {
                    o.into_mut().push(tuple);
                }
                Entry::Vacant(v) => {
                    v.insert(vec![tuple]);
                }
            };
        }

        let mut results: Option<ResultSet> = None;

        for (_typ, vals) in groups.iter() {
            // OR within the type.
            let group_result = self.features_vsm.vsm.search_by_terms(
                vals,
                0.0,
                true,
                relations::ScoreNormalization::DocA,
            );

            // First time use this group's results.
            // Subsequently, perform AND with previous groups.
            results = match results {
                Some(mut r) => {
                    r.filter_by(&group_result);
                    Some(r)
                }
                None => Some(group_result),
            };
        }

        results.unwrap_or(ResultSet::new())
    }

    // Return groups of features that we recognise.
    pub fn get_features(&self) -> &HashMap<String, Vec<String>> {
        &self.all_features_cached
    }

    pub fn get_max_tune_id(&self) -> u32 {
        self.max_tune_id
    }
}

// A user-facing result with metadata etc.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DecoratedResult {
    pub titles: Vec<String>,
    pub id: usize,
    pub score: f32,
}
