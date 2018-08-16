use std::usize;

use std::cmp::Ordering;
use std::collections::HashMap;
use std::hash::Hash;
use std::time::SystemTime;

// Provide at least this much overhead when reallocating.
pub const GROWTH_OVERHEAD: usize = 1024;

// Represent groupings of tunes.
// Tune ID usize::MAX isn't allowed.
// If we get over 4 billion tunes, it may be time to consider an Option types.
// Uses linear searches, but uses constant space and optimised for our corpus.
// For 200,000 tunes, it takes about half a millisecond to add a connection.
pub struct Grouper {
    // Dense mapping of tune id -> group ID.
    // The ID of a group is the ID of its lowest member.
    // For each tune, group ID can be:
    // - MAX  : Unassigned.
    // - Else : The tune belongs to this group ID.
    groups: Vec<usize>,
}

impl Grouper {
    pub fn new() -> Grouper {
        // Start non-empty, as we're always going to want to do something.
        // Also means len can never be zero, so we can skip a check where it matters.
        let mut groups = Vec::with_capacity(GROWTH_OVERHEAD);
        groups.resize(GROWTH_OVERHEAD, usize::MAX);

        Grouper { groups }
    }

    pub fn with_max_id(id: usize) -> Grouper {
        let mut grouper = Grouper::new();
        grouper.groups.resize(id + 1, usize::MAX);
        grouper
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
            // Start with special value of MAX.
            let mut new_id = usize::MAX;

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
pub enum ScoreNormalization {
    // Score is normalized to the length of the 'A' document.
    // Good when 'a' is a short search term.
    DocA,

    // Score is normalized to the maximum of the two document lengths.
    // Good when 'a' is a whole tune and we're looking for doc similarity.
    Max,
}

// Binary Vector Space Model, with parameterized term type.
// Allocated with static size, with each document's term vector represented as 
// a bitfield as an array of 64-bit words. The size of the bitfield is static,
// and indexes are wrapped to this size. A little like a hash table, though collisions
// are taken as part of the rough-and-tumble, so it's not possible to say exactly which
// terms are in a given document after the fact.
// Lookups are done by a linear scan over each document, with bitwise intersection and popcount.
pub struct BinaryVSM<K> {
    // Map of term to term id. This simply increments for each new temr found.
    terms: HashMap<K, usize>,

    next_term_id: usize,

    // Map of tune id -> bit vector.
    // Indexed 2d array as (tune_id * word_capacity) + term_bit
    docs_terms: Vec<u64>,

    // Top tune id
    top_id: usize,

    // Size of table per tune, recorded as bits and whole 64-bit words.
    word_capacity: usize,
    bit_capacity: usize,
}

impl<K> BinaryVSM<K>
where
    K: Eq + Hash,
{
    pub fn new(bit_capacity: usize, top_id: usize) -> BinaryVSM<K> {
        let word_capacity = bit_capacity / 64 + 1;
        eprintln!(
            "New BinaryVSM bits: {} words: {}",
            bit_capacity, word_capacity
        );

        let table = vec![0x0; word_capacity * (top_id + 1)];

        BinaryVSM {
            terms: HashMap::new(),
            docs_terms: table,
            next_term_id: 0,
            word_capacity: word_capacity,
            bit_capacity: bit_capacity,
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

        self.terms.insert(term, self.next_term_id);
        self.next_term_id += 1;
        self.next_term_id - 1
    }

    pub fn get_word_bit(&self, term_id: usize) -> (usize, usize) {
        (term_id / 64, term_id % 64)
    }

    pub fn add(&mut self, tune_id: usize, term: K) {
        if tune_id > self.top_id {
            return;
        }

        let mut term_id = self.get_term_id(term);

        // Wrap round to fit in the table.
        let bit_i = term_id % self.bit_capacity;
        let (word_offset, bit_offset) = self.get_word_bit(bit_i);
        self.docs_terms[tune_id * self.word_capacity + word_offset] |= (1 << bit_offset);
    }

    pub fn search_by_id(
        &self,
        a: usize,
        cutoff: f32,
        normalization: ScoreNormalization,
    ) -> ResultSet {
        let mut results = ResultSet::new();

        if a > (self.top_id) {
            return results;
        }

        // TODO can pull this bit out into a search_by_terms.
        let a_words =
            &self.docs_terms[self.word_capacity * (a as usize)..self.word_capacity * (a + 1)];
        let mut a_bitcount = 0;
        for word in a_words {
            a_bitcount += word.count_ones();
        }

        for b in 0..self.top_id {
            let mut b_bitcount = 0;

            let b_words = &self.docs_terms[b * self.word_capacity..(b + 1) * self.word_capacity];

            let mut num_intersecting_bits = 0;
            for i in 0..self.word_capacity {
                num_intersecting_bits += (a_words[i] & b_words[i]).count_ones();
                b_bitcount += b_words[i].count_ones();
            }

            // Different use cases call for different normalization.
            let result = match normalization {
                ScoreNormalization::DocA => (num_intersecting_bits as f32) / (a_bitcount as f32),
                ScoreNormalization::Max => {
                    (num_intersecting_bits as f32) / (u32::max(a_bitcount, b_bitcount) as f32)
                }
            };

            if a != b && num_intersecting_bits > 0 && result >= cutoff {
                results.add(b, result);
            }
        }

        results
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

    pub fn add(&mut self, tune_id: usize, interval_seq: &Vec<i16>) {
        for window in interval_seq.windows(INTERVAL_WINDOW_SIZE) {
            let mut window_arr = [0; INTERVAL_WINDOW_SIZE];
            window_arr[0] = window[0];
            window_arr[1] = window[1];
            window_arr[2] = window[2];
            window_arr[3] = window[3];
            window_arr[4] = window[4];
            self.vsm.add(tune_id, window_arr);
        }
    }
}

#[derive(Debug)]
pub struct ResultSet {
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
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn join_groups_test() {
        let mut groups = Grouper::new();

        assert_eq!(
            groups.group_ids(),
            vec![],
            "Empty grouper returns empty groups."
        );

        groups.add(1, 1);

        assert_eq!(
            groups.group_ids(),
            vec![],
            "Adding group self to self results singleton set."
        );

        // Add 1 -> 2

        groups.add(1, 2);

        assert_eq!(
            groups.group_ids(),
            vec![1],
            "Adding 1->2 results in one group with id of first."
        );

        assert_eq!(
            groups.get(1),
            groups.get(2),
            "1 and 2 are in the same group."
        );

        assert_eq!(groups.get(3), None, "3 is unknown");

        // Add 3 -> 4

        groups.add(3, 4);

        assert_eq!(
            groups.group_ids().len(),
            2,
            "Adding unrelated pair results in a second group."
        );

        assert_eq!(
            groups.get(3),
            groups.get(4),
            "3 and 4 are in the same group."
        );

        assert!(
            groups.get(1) != groups.get(3),
            "1 and 3 are not in the same group."
        );

        // Add 5 -> 6
        groups.add(5, 6);

        assert_eq!(
            groups.group_ids().len(),
            3,
            "Adding another unrelated pair results in a third group."
        );

        assert_eq!(
            groups.get(5),
            groups.get(6),
            "3 and 4 are in the same group."
        );

        assert!(
            groups.get(1) != groups.get(5),
            "1 and 5 are not in the same group."
        );

        groups.add(2, 5);

        assert_eq!(
            groups.group_ids().len(),
            2,
            "Connecting two groups reduces the number of groups."
        );

        assert_eq!(
            groups.get(2),
            groups.get(5),
            "2 and 5 are now in the same group."
        );

        assert_eq!(
            groups.get(1),
            groups.get(6),
            "1 and 6 are now in the same group."
        );

        assert!(
            groups.get(1) != groups.get(3),
            "But the second {3,4} group are still distinct from the new {1,2,4,5} group."
        );

        assert!(
            groups.get(2) != groups.get(4),
            "But the second {3,4} group are still distinct from the new {1,2,4,5} group."
        );

        // Now unify the two remaining groups into one.
        groups.add(2, 4);

        assert_eq!(
            groups.group_ids(),
            vec![1],
            "When combined, one group remains and the lowest id of all members of the group is used."
        );
    }
}
