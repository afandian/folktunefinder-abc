use std::usize;
extern crate rand;

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
            eprintln!("Group {}", group_id);
            let members = self.get_members(*group_id);
            for member in members.iter() {
                eprintln!(" - {}", member);
            }
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
