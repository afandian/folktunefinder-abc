use std::usize;

use std::collections::HashMap;
use std::collections::HashSet;
use std::fmt::Debug;
use std::hash::Hash;

extern crate glob;
extern crate time;

use std::io::Write;

use std::fs::File;
use std::io::Read;

use std::path::PathBuf;

use std::io::{BufReader, BufWriter};

use search::ResultSet;

// Provide at least this much overhead when reallocating.
pub const GROWTH_OVERHEAD: usize = 1024;

// Represent groupings of tunes.
// Tune ID usize::MAX isn't allowed.
// If we get over 4 billion tunes, it may be time to consider an Option types.
// Uses linear searches, but uses constant space and optimised for our corpus.
// For 200,000 tunes, it takes about half a millisecond to add a connection.
pub struct Clusters {
    // Dense mapping of tune id -> group ID.
    // The ID of a group is the ID of its lowest member.
    // For each tune, group ID can be:
    // - MAX  : Unassigned.
    // - Else : The tune belongs to this group ID.
    groups: Vec<usize>,
}

impl Clusters {
    pub fn new() -> Clusters {
        // Start non-empty, as we're always going to want to do something.
        // Also means len can never be zero, so we can skip a check where it matters.
        let mut groups = Vec::with_capacity(GROWTH_OVERHEAD);
        groups.resize(GROWTH_OVERHEAD, usize::MAX);

        Clusters { groups }
    }

    pub fn with_max_id(id: usize) -> Clusters {
        let mut clusters = Clusters::new();
        clusters.groups.resize(id + 1, usize::MAX);
        clusters
    }

    pub fn load(filename: &PathBuf) -> Clusters {
        let mut groups = Vec::with_capacity(GROWTH_OVERHEAD);

        if let Ok(f) = File::open(filename) {
            let mut reader = BufReader::new(f);
            let mut buf = vec![0u8; 8];
            loop {
                match reader.read_exact(&mut buf) {
                    // End of file is ok here.
                    Err(_) => break,
                    _ => (),
                }

                let value: usize = (buf[0] as usize)
                    | (buf[1] as usize) << 8
                    | (buf[2] as usize) << 16
                    | (buf[3] as usize) << 24
                    | (buf[4] as usize) << 32
                    | (buf[5] as usize) << 40
                    | (buf[6] as usize) << 48
                    | (buf[7] as usize) << 56;

                groups.push(value);
            }
        } else {
            eprintln!("No pre-existing tune cache file found, starting from scratch.");
        }

        Clusters { groups }
    }

    pub fn save(&self, filename: &PathBuf) {
        let f = File::create(filename).expect("Can't open!");
        let mut writer = BufWriter::new(f);

        let mut buf = vec![0u8; 8];

        for value in self.groups.iter() {
            let value = *value;
            // let length = buf.len();

            buf[0] = ((value & 0x00000000000000FF) >> 0) as u8;
            buf[1] = ((value & 0x000000000000FF00) >> 8) as u8;
            buf[2] = ((value & 0x0000000000FF0000) >> 16) as u8;
            buf[3] = ((value & 0x00000000FF000000) >> 24) as u8;
            buf[4] = ((value & 0x000000FF00000000) >> 32) as u8;
            buf[5] = ((value & 0x0000FF0000000000) >> 40) as u8;
            buf[6] = ((value & 0x00FF000000000000) >> 48) as u8;
            buf[7] = ((value & 0xFF00000000000000) >> 52) as u8;

            writer.write_all(&buf).expect("Can't write");
        }
    }

    // Merge this group by the content of the other.
    pub fn extend(&mut self, other: Clusters) {
        // We know about the internals of the other one, so we can take a shortcut.
        // The index of the array is the 'a' id, the value 'b' id.
        for a in 0..other.groups.len() {
            let b = other.groups[a];
            if b != usize::MAX {
                self.add(a, b);
            }
        }
    }

    // Put A and B into the same group.
    pub fn add(&mut self, a: usize, b: usize) {
        if a == b || a == usize::MAX || b == usize::MAX {
            return;
        }

        // len() - 1 is ok because the vector is initialized non-empty and never shrinks.

        // Ensure that both IDs are represented.
        if a > self.groups.len() - 1 {
            self.groups.resize(a + 1 + GROWTH_OVERHEAD, usize::MAX);
        }

        if b > self.groups.len() - 1 {
            self.groups.resize(b + 1 + GROWTH_OVERHEAD, usize::MAX);
        }

        if self.groups[a] == usize::MAX && self.groups[b] == usize::MAX {
            // If neither is in a group yet, create a new one.
            // Choose A's ID as being the group ID.

            // Set marker on A to say that it's the canonical ID for this group.
            self.groups[a] = a;
            // B is the second member of the A group.
            self.groups[b] = a;
        } else if self.groups[a] == usize::MAX && self.groups[b] != usize::MAX {
            // If A isn't in a group but B is, add A to B's group.
            self.groups[a] = self.groups[b];
        } else if self.groups[a] != usize::MAX && self.groups[b] == usize::MAX {
            // And vice versa...
            self.groups[b] = self.groups[a];
        } else {
            // Otherwise B and A are in different groups. Unite them.

            let old_group_a: usize = self.groups[a];
            let old_group_b: usize = self.groups[b];

            // Update all members of A and B's previous groups to be A value.

            // Choose a new ID for the group. This will be the lowest ID we find.
            let mut new_id = usize::min(a, b);

            for i in 0..self.groups.len() {
                if self.groups[i] == old_group_a || self.groups[i] == old_group_b {
                    // First time we see a member of either group, use that as the new group id.
                    if new_id == usize::MAX {
                        new_id = i;
                    } else {
                        self.groups[i] = new_id;
                    }
                }
            }
        }
    }

    // Get the group ID of a given id.
    pub fn get(&self, a: usize) -> Option<usize> {
        if let Some(group_id) = self.groups.get(a) {
            if *group_id == usize::MAX {
                // Not in a group.
                None
            } else {
                // Any other number is the actual group ID.
                Some(*group_id)
            }
        } else {
            None
        }
    }

    // Allocate and return a vector of tune IDs.
    pub fn group_ids(&self) -> Vec<usize> {
        let mut result = vec![];

        // Scan for the first member of each group whose group is the same as the id.
        for (i, value) in self.groups.iter().enumerate() {
            if *value == i {
                result.push(i);
            }
        }

        result
    }

    pub fn num_groups(&self) -> usize {
        let mut result = 0;

        for (i, value) in self.groups.iter().enumerate() {
            if *value == i {
                result += 1;
            }
        }

        result
    }

    // Allocate and return list of members of group.
    pub fn get_members(&self, a: usize) -> Vec<usize> {
        let mut result = vec![];

        // Scan for canonical group ids.
        for (i, value) in self.groups.iter().enumerate() {
            if *value == a {
                result.push(i);
            }
        }

        result
    }

    pub fn print_debug(&self) {
        let groups = self.group_ids();
        for group_id in groups.iter() {
            let members = self.get_members(*group_id);
            // Print to STDOUT as this is useful.
            println!("{:?}", members);
        }
    }

    // Return seq of groups. Could allocate a lot, really designed for testing.
    pub fn get_groups(&self) -> Vec<Vec<usize>> {
        let mut result = vec![];
        let groups = self.group_ids();
        for group_id in groups.iter() {
            result.push(self.get_members(*group_id));
        }
        result
    }

    // Find the next tune after this ID that isn't assigned to a group.
    // This relies on having been constructed with a max tune id so it knows about all the potential IDs.
    pub fn next_ungrouped_after(&self, a: u32) -> Option<usize> {
        for i in (a + 1) as usize..self.groups.len() {
            if self.groups[i] == usize::MAX {
                return Some(i);
            }
        }

        None
    }
}

// Defines how score normalization for similarity should be done when comparing two documents.
#[derive(Clone, Copy)]
pub enum ScoreNormalization {
    // Score is normalized to the length of the 'A' document.
    // Good when 'a' is a short search term.
    DocA,

    // Score is normalized to the maximum of the two document lengths.
    // Good when 'a' is a whole tune and we're looking for doc similarity.
    Max,
}

impl ScoreNormalization {
    pub fn score(&self, num_intersecting_bits: u32, a_bitcount: u32, b_bitcount: u32) -> f32 {
        match self {
            ScoreNormalization::DocA => (num_intersecting_bits as f32) / (a_bitcount as f32),
            ScoreNormalization::Max => {
                (num_intersecting_bits as f32) / (u32::max(a_bitcount, b_bitcount) as f32)
            }
        }
    }
}

// Binary Vector Space Model, with parameterized term type.
// Allocated with static size, with each document's term vector represented as
// a bitfield as an array of 64-bit words. The size of the bitfield is static,
// and indexes are wrapped to this size. A little like a hash table, though collisions
// are taken as part of the rough-and-tumble, so it's not possible to say exactly which
// terms are in a given document after the fact.
// Lookups are done by a linear scan over each document, with bitwise intersection and popcount.
pub struct BinaryVSM<K> {
    // Map of term to term id. This simply increments for each new term found.
    terms: HashMap<K, usize>,

    // Inverted map of term to value.
    terms_i: HashMap<usize, K>,

    next_term_id: usize,

    // Map of tune id -> bit vector.
    // Indexed 2d array as (tune_id * word_capacity) + term_bit
    docs_terms: Vec<u64>,

    // Map of tune id -> list of term IDs found.
    pub docs_terms_exact: Vec<HashSet<usize>>,

    // Top tune id
    top_id: usize,

    // Size of table per tune, recorded as bits and whole 64-bit words.
    word_capacity: usize,
    bit_capacity: usize,
}

impl<K> BinaryVSM<K>
where
    K: Eq + Hash + Clone + Debug + Ord,
{
    pub fn new(bit_capacity: usize, top_id: usize) -> BinaryVSM<K> {
        let word_capacity = bit_capacity / 64 + 1;
        eprintln!(
            "New BinaryVSM bits: {} words: {} top tune id: {}",
            bit_capacity, word_capacity, top_id
        );

        let table = vec![0x0; word_capacity * (top_id + 1)];
        let exact = vec![HashSet::new(); top_id + 1];

        BinaryVSM {
            terms: HashMap::new(),
            terms_i: HashMap::new(),
            docs_terms: table,
            next_term_id: 0,
            word_capacity: word_capacity,
            bit_capacity: bit_capacity,
            docs_terms_exact: exact,
            top_id: top_id,
        }
    }

    // Term to Term ID.
    // This is a number in an unbounded range.
    // It will later be modded to fit in the term bitfield.
    pub fn get_term_id(&mut self, term: K) -> usize {
        if let Some(id) = self.terms.get(&term) {
            return *id;
        }

        let id = self.next_term_id;
        // We need to store the term twice.
        // TODO can we store a reference to the `terms` heap object instead?
        self.terms_i.insert(id, term.clone());
        self.terms.insert(term, id);
        self.next_term_id += 1;

        id
    }

    pub fn get_word_bit(&self, term_id: usize) -> (usize, usize) {
        (term_id / 64, term_id % 64)
    }

    // TODO could somehow merge add and search_by_terms.
    pub fn add(&mut self, tune_id: usize, term: K) {
        if tune_id > self.top_id {
            return;
        }

        // Squirrel away a copy of this in the lookup table.
        // This is so we can use things like Strings, which can't be simply copied.
        let term_id = self.get_term_id(term.clone().to_owned());

        // Wrap round to fit in the table.
        let bit_i = term_id % self.bit_capacity;
        let (word_offset, bit_offset) = self.get_word_bit(bit_i);
        self.docs_terms[tune_id * self.word_capacity + word_offset] |= 1 << bit_offset;
        self.docs_terms_exact[tune_id].insert(term_id);
    }

    // TODO can terms be a ref?
    pub fn search_by_terms(
        &self,
        terms: Vec<K>,
        cutoff: f32,
        exact: bool,
        normalization: ScoreNormalization,
    ) -> ResultSet {
        eprintln!("Search by terms: {:?}", terms);

        // Set of term IDs as a bit vector.
        let mut words = vec![0; self.word_capacity];

        // Set of term IDs as a set.
        let mut terms_set: HashSet<usize> = HashSet::new();

        // Set bits for terms.
        for term in terms.iter() {
            // If the term doesn't exist, just ignore.
            if let Some(term_id) = self.terms.get(&term) {
                let bit_i = term_id % self.bit_capacity;
                let (word_offset, bit_offset) = self.get_word_bit(bit_i);
                words[word_offset] |= 1 << bit_offset;

                if exact {
                    terms_set.insert(*term_id);
                }
            }
        }

        self.search_by_bitfield_words(
            &words,
            cutoff,
            if exact { Some(terms_set) } else { None },
            normalization,
        )
    }

    // Search by a bit vector of term IDs. This is lossy, as there can be some wrapping.
    // If an optional HashSet of term IDs is supplied, scope down results exactly to that.
    pub fn search_by_bitfield_words(
        &self,
        a_words: &[u64],
        cutoff: f32,
        exact_terms: Option<HashSet<usize>>,
        normalization: ScoreNormalization,
    ) -> ResultSet {
        let mut results = ResultSet::new();

        let mut a_bitcount = 0;
        for word in a_words {
            a_bitcount += word.count_ones();
        }

        // Full scan of each document's bit vector.
        // A is the query document. B is the other document (we're scanning).
        for b in 0..self.top_id {
            let mut b_bitcount = 0;

            let b_words = &self.docs_terms[b * self.word_capacity..(b + 1) * self.word_capacity];

            let mut num_intersecting_bits = 0;
            for i in 0..self.word_capacity {
                num_intersecting_bits += (a_words[i] & b_words[i]).count_ones();
                b_bitcount += b_words[i].count_ones();
            }

            // If nothing intersects, that's zero match.
            if num_intersecting_bits == 0 {
                continue;
            }

            // Get an initial score based on the lossy bit intsection.
            let score = normalization.score(num_intersecting_bits, a_bitcount, b_bitcount);

            // If the score doesn't match the threshold, stop.
            // If we do further exact matching, we may end up with a higher score. However, putting
            // this guard here makes sense.
            // Firstly, if there's no exact matching, it needs to happen anyway.
            // Secondly, if there is exact matching, it's more expensive, so we need to guard it.
            if score < cutoff {
                continue;
            }

            match exact_terms {
                // If already matches because of the above guard.
                None => {
                    results.add(b, score);
                }
                // Need to do a further test.
                Some(ref a_term_ids) => {
                    if let Some(b_term_ids) = self.docs_terms_exact.get(b) {
                        let intersecting_values =
                            a_term_ids.intersection(b_term_ids).count() as u32;
                        let exact_score =
                            normalization.score(intersecting_values, a_bitcount, b_bitcount);

                        if intersecting_values > 0 && score >= cutoff {
                            results.add(b, exact_score);
                        }
                    }
                }
            };
        }

        results
    }

    // Search by similarity to an existing document's term vector.
    pub fn search_by_id(
        &self,
        a: usize,
        cutoff: f32,
        normalization: ScoreNormalization,
    ) -> ResultSet {
        let results = ResultSet::new();

        if a > (self.top_id) {
            return results;
        }

        let a_words =
            &self.docs_terms[self.word_capacity * (a as usize)..self.word_capacity * (a + 1)];
        self.search_by_bitfield_words(a_words, cutoff, None, normalization)
    }

    pub fn print_debug_tunes(&self) {
        for id in 0..self.top_id {
            if self.docs_terms_exact[id].len() > 0 {
                eprintln!("Doc {}:", id);
                for term in self.docs_terms_exact[id].iter() {
                    eprint!("{:?} ", self.terms_i.get(term));
                }
                eprintln!("");
            }
        }
    }
}

const INTERVAL_WINDOW_SIZE: usize = 5;

// Binary Vector Space model, each term being a sliding window over the interval sequence.
pub struct IntervalWindowBinaryVSM {
    pub vsm: BinaryVSM<[i16; INTERVAL_WINDOW_SIZE]>,
}

impl IntervalWindowBinaryVSM {
    pub fn new(size: usize, top_id: usize) -> IntervalWindowBinaryVSM {
        IntervalWindowBinaryVSM {
            vsm: BinaryVSM::new(size, top_id),
        }
    }

    // TODO this allocates an interrim vec so it can be reused.
    // Could somehow do this as an interator?
    fn intervals_to_terms(interval_seq: &[i16]) -> Vec<[i16; INTERVAL_WINDOW_SIZE]> {
        let mut terms = vec![];

        for window in interval_seq.windows(INTERVAL_WINDOW_SIZE) {
            let mut window_arr = [0; INTERVAL_WINDOW_SIZE];
            window_arr[0] = window[0];
            window_arr[1] = window[1];
            window_arr[2] = window[2];
            window_arr[3] = window[3];
            window_arr[4] = window[4];
            terms.push(window_arr);
        }

        terms
    }

    pub fn add(&mut self, tune_id: usize, interval_seq: &Vec<i16>) {
        for term in IntervalWindowBinaryVSM::intervals_to_terms(&interval_seq) {
            self.vsm.add(tune_id, term);
        }
    }

    pub fn search(
        &self,
        interval_seq: &Vec<i16>,
        cutoff: f32,
        normalization: ScoreNormalization,
    ) -> ResultSet {
        let terms = IntervalWindowBinaryVSM::intervals_to_terms(interval_seq);

        self.vsm
            .search_by_terms(terms, cutoff, false, normalization)
    }
}

pub struct FeaturesBinaryVSM {
    pub vsm: BinaryVSM<(String, String)>,
}

impl FeaturesBinaryVSM {
    pub fn new(size: usize, top_id: usize) -> FeaturesBinaryVSM {
        FeaturesBinaryVSM {
            vsm: BinaryVSM::new(size, top_id),
        }
    }

    pub fn add(&mut self, tune_id: usize, feature_type: String, value: String) {
        self.vsm.add(tune_id, (feature_type, value));
    }

    // Print out features.
    pub fn debug_print_features(&self) {
        let mut all_terms: Vec<&(String, String)> = self.vsm.terms.keys().collect();
        all_terms.sort();
        let mut prev_feature_type = &"".to_string();
        for (feature_type, feature_value) in all_terms.iter() {
            // In stream of (type, feature) Detect change in type.
            if feature_type != prev_feature_type {
                eprintln!("{}", feature_type);
                prev_feature_type = feature_type;
            }
            eprintln!("  {}", feature_value);
        }
    }

    // Return structure of all known feature values, grouped by type.
    pub fn all_features(&self) -> HashMap<String, Vec<String>> {
        let mut results: HashMap<String, Vec<String>> = HashMap::new();

        let mut all_terms: Vec<&(String, String)> = self.vsm.terms.keys().collect();
        all_terms.sort();
        let mut prev_feature_type = &"".to_string();
        let mut vals = vec![];
        for (feature_type, feature_value) in all_terms.iter() {
            // In stream of (type, feature) Detect change in type.
            if feature_type != prev_feature_type {
                if vals.len() > 0 {
                    results.insert(prev_feature_type.to_string(), vals);
                }
                vals = vec![];
                prev_feature_type = feature_type;
            }

            vals.push(feature_value.to_string());
        }

        if vals.len() > 0 {
            results.insert(prev_feature_type.to_string(), vals);
        }
        results
    }

    // Return structure of feature types, values and counts for the Result Set.
    // Used for faceting.
    // Structure is feature-type -> feature-value -> count
    pub fn facet_features_for_resultset(
        &self,
        result_set: &ResultSet,
    ) -> HashMap<String, Vec<(String, u32)>> {
        let mut counts: HashMap<(String, String), u32> = HashMap::new();

        for (tune_id, _) in result_set.results.iter() {
            let feature_ids = &self.vsm.docs_terms_exact[*tune_id];
            for feature_id in feature_ids.iter() {
                if let Some((ref typ, ref val)) = self.vsm.terms_i.get(feature_id) {
                    *counts
                        .entry((typ.to_string(), val.to_string()))
                        .or_insert(0) += 1;
                }
            }
        }

        let mut results = HashMap::new();
        for ((typ, val), cnt) in counts {
            (*results.entry(typ.to_string()).or_insert(vec![])).push((val, cnt));
        }

        // Within each type, sort by count.
        // No limiting just yet. With current data, a full page of all facet values is 200Kb,
        // which is fine. If this gets fed into a user interface, some kind of limiting should
        // happen downstream.
        for (_k, v) in results.iter_mut() {
            v.sort_by(|(_, cnt_a), (_, cnt_b)| cnt_b.cmp(cnt_a));
        }

        results
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extend_groups_test() {
        // Three chunks.
        // A: (1, 2, 3) and (4, 5, 6) and (7, 8, 9)
        // B: (3, 4) and (10, 11, 12), (13, 14)
        // C: (6, 12)

        let a = vec![(1, 2), (2, 3), (4, 5), (4, 6), (7, 8), (8, 9)];
        let b = vec![(3, 4), (10, 11), (11, 12), (13, 14)];
        let c = vec![(6, 12)];

        let mut groups_a = Clusters::with_max_id(15);
        for (x, y) in a {
            groups_a.add(x, y);
        }
        let mut groups_b = Clusters::with_max_id(15);
        for (x, y) in b {
            groups_b.add(x, y);
        }
        let mut groups_c = Clusters::with_max_id(15);
        for (x, y) in c {
            groups_c.add(x, y);
        }

        // Check that all groups were built properly.
        assert_eq!(
            groups_a.get_groups(),
            vec![
                vec![1usize, 2usize, 3usize],
                vec![4usize, 5usize, 6usize],
                vec![7usize, 8usize, 9usize],
            ]
        );
        assert_eq!(
            groups_b.get_groups(),
            vec![
                vec![3usize, 4usize],
                vec![10usize, 11usize, 12usize],
                vec![13usize, 14usize],
            ]
        );
        assert_eq!(groups_c.get_groups(), vec![vec![6usize, 12usize]]);

        // Now merge them one by one
        let mut groups_all = Clusters::with_max_id(15);

        // Starting with an empty group.
        assert_eq!(groups_all.get_groups(), vec![] as Vec<Vec<usize>>);

        // Extend empty with A, should equal A.
        groups_all.extend(groups_a);
        assert_eq!(
            groups_all.get_groups(),
            vec![
                vec![1usize, 2usize, 3usize],
                vec![4usize, 5usize, 6usize],
                vec![7usize, 8usize, 9usize],
            ]
        );

        // Further extend with B.
        // The (1, 2, 3) and (3, 4) and (4, 5, 6) should have merged into (1, 2, 3, 4, 5, 6)
        // (7, 8, 9) and (10, 11, 12) and (13, 14) should be separate.
        groups_all.extend(groups_b);
        assert_eq!(
            groups_all.get_groups(),
            vec![
                vec![1usize, 2usize, 3usize, 4usize, 5usize, 6usize],
                vec![7usize, 8usize, 9usize],
                vec![10usize, 11usize, 12usize],
                vec![13usize, 14usize],
            ]
        );

        // Further extend with C, which should connect groups.
        // (1, 2, 3, 4, 5, 6, 10, 11, 12) with (13, 14) still separate
        groups_all.extend(groups_c);
        assert_eq!(
            groups_all.get_groups(),
            vec![
                vec![1usize, 2usize, 3usize, 4usize, 5usize, 6usize, 10usize, 11usize, 12usize,],
                vec![7usize, 8usize, 9usize],
                vec![13usize, 14usize],
            ]
        );
    }

    #[test]
    fn join_groups_test() {
        let mut groups = Clusters::new();

        assert_eq!(
            groups.get_groups(),
            vec![] as Vec<Vec<usize>>,
            "Empty Clusters returns empty groups."
        );

        groups.add(1, 1);

        assert_eq!(
            groups.get_groups(),
            vec![] as Vec<Vec<usize>>,
            "Adding group self to self results in nothing."
        );

        // Add 1 -> 2

        groups.add(1, 2);

        assert_eq!(
            groups.get_groups(),
            vec![vec![1usize, 2usize]],
            "Adding 1->2 results in one with id of first."
        );

        // Add 3 -> 4

        groups.add(3, 4);

        assert_eq!(
            groups.get_groups(),
            vec![vec![1usize, 2usize], vec![3usize, 4usize]],
            "Adding unrelated pair results in a second group."
        );

        // Add 5 -> 6
        groups.add(5, 6);

        assert_eq!(
            groups.get_groups(),
            vec![
                vec![1usize, 2usize],
                vec![3usize, 4usize],
                vec![5usize, 6usize],
            ],
            "Adding unrelated pair results in a second group."
        );

        groups.add(2, 5);

        assert_eq!(
            groups.get_groups(),
            vec![vec![1usize, 2usize, 5usize, 6usize], vec![3usize, 4usize]],
            "Connecting two groups reduces the number of groups."
        );
        // Now unify the two remaining groups into one.
        groups.add(2, 4);

        assert_eq!(
            groups.get_groups(),
            vec![vec![1usize, 2usize, 3usize, 4usize, 5usize, 6usize]],
            "Connecting two groups reduces the number of groups."
        );
    }
}
