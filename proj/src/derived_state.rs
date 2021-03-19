use crate::spec::{MethodName, PartHeads, Spec};
use proj_core::{run_len, Row, Stage};
use serde::Serialize;
use std::collections::{HashMap, HashSet};
use std::rc::Rc;

// Imports used only for the doc comments
#[allow(unused_imports)]
use crate::spec::Frag;

/// A small datatype that represents **where** a given row comes from in the composition.  This is
/// useful because the composition contains many fragments, and each row of each fragment could
/// expand into multiple actual rows (one for each part).
#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub struct RowOrigin {
    /// The index of the part that this [`Row`] came from
    part: usize,
    /// The index of the fragment that this [`Row`] came from
    frag: usize,
    /// The index of the row within the fragment that this [`Row`] came from
    row: usize,
}

impl RowOrigin {
    /// Creates a new `RowOrigin` from it's parts
    pub fn new(part: usize, frag: usize, row: usize) -> RowOrigin {
        RowOrigin { part, frag, row }
    }
}

/// A small datatype that represents **where** a given row comes from in the composition (without
/// knowledge of which part this comes from).
#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct RowLocation {
    /// The index of the fragment that this [`Row`] came from
    frag: usize,
    /// The index of the row within the fragment that this [`Row`] came from
    row: usize,
}

impl RowLocation {
    /// Creates a new `RowOrigin` from it's parts
    pub fn new(frag: usize, row: usize) -> Self {
        RowLocation { frag, row }
    }
}

impl From<RowOrigin> for RowLocation {
    fn from(o: RowOrigin) -> Self {
        RowLocation {
            frag: o.frag,
            row: o.row,
        }
    }
}

/// All the information required for JS to render a single [`Row`] from the [`Spec`].  Note that
/// because of multipart expansion, this single on-screen [`Row`] actually represents many expanded
/// [`Row`]s, and this datatype reflects that.
#[derive(Serialize, Debug, Clone)]
pub struct ExpandedRow {
    #[serde(skip_serializing_if = "Option::is_none")]
    call_str: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    method_str: Option<MethodName>,
    #[serde(skip_serializing_if = "crate::ser_utils::is_false")]
    is_lead_end: bool,
    #[serde(skip_serializing_if = "crate::ser_utils::is_true")]
    is_proved: bool,
    /// One [`Row`] for each part of the composition
    #[serde(serialize_with = "crate::ser_utils::ser_rows")]
    rows: Vec<Row>,
    /// For each bell, shows which parts contain music
    ///
    /// E.g. for `21345678` under part heads `12345678, 18234567, ...` would form rows
    /// ```text
    /// 0: 21345678
    /// 1: 81234567
    /// 2: 71823456
    /// 3: 61782345
    /// 4: 51678234
    /// 5: 41567823
    /// 6: 31456782
    /// ```
    /// and the highlights would be:
    /// ```ignore
    /// vec![
    ///     vec![],
    ///     vec![1],
    ///     vec![0, 1],
    ///     vec![0, 1, 2],
    ///     vec![0, 1, 2, 3],
    ///     vec![0, 1, 2, 3],
    ///     vec![0, 1, 2, 3],
    ///     vec![0, 1, 2, 3]
    /// ]
    /// ```
    #[serde(skip_serializing_if = "crate::ser_utils::is_all_empty")]
    music_highlights: Vec<Vec<usize>>,
}

impl ExpandedRow {
    fn calculate_music(all_rows: &[Row], stage: Stage) -> Vec<Vec<usize>> {
        // Initialise the music scores with 0 for every place
        let mut music = vec![Vec::new(); stage.as_usize()];
        // For each part that contains music, add one to the bells which are covered by the music
        for (part, r) in all_rows.iter().enumerate() {
            // Highlight runs of >=4 bells of the **front**
            let run_len_f = run_len(r.bells());
            if run_len_f >= 4 {
                music[..run_len_f].iter_mut().for_each(|m| m.push(part));
            }
            // Highlight runs of >=4 bells of the **back**
            let run_len_b = run_len(r.bells().rev());
            if run_len_b >= 4 {
                // The 'max' prevents the two ranges from overlapping and causing music in multiple
                // runs from being counted twice
                music[(stage.as_usize() - run_len_b).max(run_len_f)..]
                    .iter_mut()
                    .for_each(|m| m.push(part));
            }
        }
        music
    }

    /// Create a new `ExpandedRow` from its constituent parts
    pub fn new(
        row: &Row,
        call_str: Option<String>,
        method_str: Option<MethodName>,
        is_lead_end: bool,
        part_heads: &[Row],
        is_proved: bool,
    ) -> Self {
        let all_rows: Vec<Row> = part_heads.iter().map(|ph| ph * row).collect();
        ExpandedRow {
            call_str,
            method_str,
            is_lead_end,
            music_highlights: Self::calculate_music(&all_rows, row.stage()),
            rows: all_rows,
            is_proved,
        }
    }
}

/// A range of rows which should be highlighted as all false in the same way.  This is supposed to
/// cover `start..=end` rows (i.e. the ranges are **inclusive**).
#[derive(Serialize, Debug, Clone)]
pub struct FalseRowRange {
    start: usize,
    end: usize,
    group: usize,
}

/// A struct determining which linking groups the top and bottom of a [`Frag`] belongs to.  This
/// will determine what colour line will be displayed on each end of the [`Frag`], to make round
/// blocks detectable.
#[derive(Serialize, Debug, Clone, Default)]
pub struct FragLinkGroups {
    #[serde(skip_serializing_if = "Option::is_none")]
    link_group_top: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    link_group_bottom: Option<usize>,
}

/// The information required for JS to render a [`Frag`]
#[derive(Serialize, Debug, Clone)]
pub struct AnnotFrag {
    false_row_ranges: Vec<FalseRowRange>,
    exp_rows: Vec<ExpandedRow>,
    is_proved: bool,
    #[serde(flatten)]
    link_groups: FragLinkGroups,
    x: f32,
    y: f32,
}

/// General statistics about the composition, to be displayed in the top-left corner of the screen
#[derive(Serialize, Debug, Clone)]
pub struct DerivedStats {
    part_len: usize,
    num_false_rows: usize,
    num_false_groups: usize,
}

/// A struct that says that [`Frag`] #`to` can be linked onto the end of [`Frag`] #`from`.  This
/// will be stored in a `Vec`, representing a non-symmetric relation over the [`Frag`]s in the
/// composition.
#[derive(Serialize, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct FragLink {
    from: usize,
    to: usize,
    group: usize,
}

#[derive(Serialize, Debug, Clone)]
pub struct DerivedState {
    annot_frags: Vec<AnnotFrag>,
    frag_links: Vec<FragLink>,
    stats: DerivedStats,
    #[serde(flatten)]
    part_heads: Rc<PartHeads>,
    stage: usize,
}

impl DerivedState {
    /// Gets the [`Row`] at a given location in this `DerivedState`, returning `None` if the
    /// location doesn't correspond to a [`Row`].
    pub fn get_row(&self, part_ind: usize, frag_ind: usize, row_ind: usize) -> Option<&Row> {
        Some(
            self.annot_frags
                .get(frag_ind)?
                .exp_rows
                .get(row_ind)?
                .rows
                .get(part_ind)?,
        )
    }

    /// Gets the part head at a given index, or returning `None` if the index is bigger than the
    /// number of parts.
    #[inline]
    pub fn get_part_head(&self, part_ind: usize) -> Option<&Row> {
        self.part_heads.rows().get(part_ind)
    }

    /// Given a [`Spec`]ification, derive a new `DerivedState` from it.
    pub fn from_spec(spec: &Spec) -> DerivedState {
        // Fully expand the comp from the [`Spec`]
        let (generated_rows, part_heads) = spec.expand();

        // Truth proving pipeline
        let (flat_proved_rows, part_len) = Self::flatten_proved_rows(&generated_rows, spec.len());
        let (false_rows, num_false_rows) = Self::gen_false_row_groups(flat_proved_rows);
        let (mut ranges_by_frag, num_false_groups) = Self::coalesce_false_row_groups(false_rows);

        // Decide how the frags link together
        let (frag_links, frag_link_groups) = Self::gen_frag_links(&generated_rows);

        // Compile all of the derived state into one struct
        DerivedState {
            frag_links,
            part_heads,
            annot_frags: generated_rows
                .into_iter()
                .zip(frag_link_groups.into_iter())
                .enumerate()
                .map(|(i, (exp_rows, link_groups))| {
                    // Sanity check that leftover rows should never be used in the proving
                    assert!(exp_rows.last().map_or(false, |r| !r.is_proved));
                    let (x, y) = spec.frag_pos(i).unwrap();
                    AnnotFrag {
                        false_row_ranges: ranges_by_frag.remove(&i).unwrap_or_default(),
                        exp_rows,
                        is_proved: !spec.is_frag_muted(i).unwrap(),
                        link_groups,
                        x,
                        y,
                    }
                })
                .collect(),
            stats: DerivedStats {
                part_len,
                num_false_groups,
                num_false_rows,
            },
            stage: spec.stage().as_usize(),
        }
    }

    /// Given the expanded rows from each [`Frag`] of the composition, find which pairs of
    /// [`Frag`]s should be connected (i.e. [`Frag`]s (x, y) are linked x -> y iff the leftover row
    /// of x is the same as the first row of y).  This is then used to determine which [`Frag`]s
    /// can be joined together.  This also calculates which groups the top and bottom of each
    /// [`Frag`] belongs to
    fn gen_frag_links(generated_rows: &[Vec<ExpandedRow>]) -> (Vec<FragLink>, Vec<FragLinkGroups>) {
        let num_frags = generated_rows.len();
        // A map to determine which group ID should be assigned to each Row.  This way,
        // interconnected groups of links are given the same colours.
        let mut link_groups: HashMap<&Row, usize> = HashMap::new();
        let mut frag_links = Vec::new();
        let mut frag_link_groups = vec![FragLinkGroups::default(); num_frags];

        // Test every pair of frags f -> g ...
        for (i, f) in generated_rows.iter().enumerate() {
            for (j, g) in generated_rows.iter().enumerate() {
                // ... if `g` starts with the leftover row of `f`, then f -> g ...
                let leftover_row_of_f = &f.last().unwrap().rows[0];
                let first_row_of_g = &g[0].rows[0];
                if leftover_row_of_f == first_row_of_g {
                    // Decide what group this link should be put in (so that all the links of the
                    // same row get coloured the same colour).
                    let link_groups_len = link_groups.len();
                    let group = *link_groups
                        .entry(leftover_row_of_f)
                        .or_insert(link_groups_len);
                    // Add the frag links, and assign the frag tip colours
                    frag_links.push(FragLink {
                        from: i,
                        to: j,
                        group,
                    });
                    frag_link_groups[i].link_group_bottom = Some(group);
                    frag_link_groups[j].link_group_top = Some(group);
                }
            }
        }
        (frag_links, frag_link_groups)
    }

    /// Take a jagged array of `ExpandedRow`s, and return all the [`Row`]s that should be
    /// proven, along with their origin.  This also returns the number of proven rows from each
    /// part.  This does **not** sort the flattened rows.
    fn flatten_proved_rows(
        generated_rows: &[Vec<ExpandedRow>],
        spec_len: usize,
    ) -> (Vec<(RowOrigin, &Row)>, usize) {
        // Expand all the rows and their origins from the composition into a `Vec` to be
        // proved, excluding the last Row of each Frag, since that is 'left over' and as such
        // shouldn't be used of proving
        let mut flattened_rows: Vec<(RowOrigin, &Row)> = Vec::with_capacity(spec_len);
        let mut part_len = 0;
        for (frag_index, rows) in generated_rows.iter().enumerate() {
            for (row_index, expanded_row) in rows.iter().enumerate() {
                // Only prove rows if they should be proven
                if !expanded_row.is_proved {
                    continue;
                }
                for (part_index, row) in expanded_row.rows.iter().enumerate() {
                    flattened_rows.push((RowOrigin::new(part_index, frag_index, row_index), row));
                }
                // Count the single ExpandedRow as one row per part (despite it expanding to
                // several individual Rows)
                part_len += 1;
            }
        }
        (flattened_rows, part_len)
    }

    /// Given the flattened rows of a composition, group the rows on the screen into false groups
    /// (note that these are groups of individual rows which are the same, rather than the
    /// 'meta-groups' that the user sees).  `spec_len` is used to make sure that we allocate
    /// exactly the right amount of space when flattening the rows
    fn gen_false_row_groups(
        mut flattened_rows: Vec<(RowOrigin, &Row)>,
    ) -> (Vec<Vec<RowLocation>>, usize) {
        // Sort all_rows only by their rows, so that false rows are appear next to each other.  The
        // algorithm won't work unless the input rows are sorted.
        flattened_rows.sort_by(|(_, r1), (_, r2)| r1.cmp(r2));

        // We use a hashset because if the part heads form a group then any falseness will be the
        // same between all the parts, so will appear lots of times.
        let mut false_rows: HashSet<Vec<RowLocation>> = HashSet::new();
        // The origins of the current set of duplicated rows.  Most of the time, we hope that
        // this has length 1, i.e. all rows are unique.
        let mut current_false_row_group: Vec<RowLocation> = Vec::with_capacity(10);
        let mut last_row = None;
        let mut num_false_rows = 0usize;
        // Iterate over all the rows, compiling groups as we go
        for (o, r) in flattened_rows.iter() {
            if let Some(l_r) = &last_row {
                if l_r != &r {
                    // If we reach this branch of the code, then it means that we are just
                    // starting a new set of rows and we need to check the last group for
                    // falseness.
                    if current_false_row_group.len() > 1 {
                        // Add these rows to the falseness counter
                        num_false_rows += current_false_row_group.len();
                        // If we saw more than 1 identical rows, then this counts as falseness
                        // and so we add this to the set of false rows.  We sort the row
                        // locations first, to make sure that if the same set of `RowLocation`s
                        // is found twice they are always added once (regardless of which order
                        // the rows were discovered).
                        current_false_row_group.sort();
                        false_rows.insert(std::mem::take(&mut current_false_row_group));
                    } else {
                        current_false_row_group.clear();
                    }
                }
            }
            // Make sure that the current row becomes the last row for the next iteration, and
            // add this location to the current group.
            last_row = Some(r);
            current_false_row_group.push(RowLocation::from(*o));
        }
        // Make sure that we don't miss the last false row group
        if current_false_row_group.len() > 1 {
            // Add these rows to the falseness counter
            num_false_rows += current_false_row_group.len();
            // If we saw more than 1 identical rows, then this counts as falseness and so we
            // add this to the set of false rows.  We sort the row locations first, to make
            // sure that if the same set of `RowLocation`s is found twice they are always added
            // once (regardless of which order the rows were discovered).
            current_false_row_group.sort();
            false_rows.insert(current_false_row_group);
        }
        // Convert the HashMap into a Vec (without cloning), and return it along with the number of
        // false rows
        (false_rows.into_iter().collect::<Vec<_>>(), num_false_rows)
    }

    /// Combine adjacent false row groups so that we use up fewer colours.  This relies on the
    /// fact that all the `Vec`s in `false_rows` are sorted in increasing order by frag index and
    /// then row index (and a unit test checks that).
    fn coalesce_false_row_groups(
        mut false_rows: Vec<Vec<RowLocation>>,
    ) -> (HashMap<usize, Vec<FalseRowRange>>, usize) {
        let mut ranges_by_frag: HashMap<usize, Vec<FalseRowRange>> = HashMap::new();
        // Firstly, convert the existing HashSet into a Vec, and sort it
        false_rows.sort();
        // Because `false_rows_vec` itself and all its contents are sorted, we can guaruntee
        // that the rows making up each false row group are sequential in the listing
        let mut iter = false_rows.iter();
        let mut group_id = 0;
        if let Some(first_group) = iter.next() {
            let mut last_group = first_group;
            let mut first_group_in_meta_group = first_group;
            for group in iter {
                // Decide if this group is adjacent to the last one (for two groups to be
                // adjacent, they need to have the same length and all the `RowLocation`s must
                // also be adjacent -- we don't worry about the order of each group because
                // they have all been sorted the same way so a simple zipping check will
                // suffice).
                let is_adjacent_to_last = group.len() == last_group.len()
                    && group.iter().zip(last_group.iter()).all(|(loc1, loc2)| {
                        loc1.frag == loc2.frag && (loc1.row as isize - loc2.row as isize).abs() == 1
                    });
                if !is_adjacent_to_last {
                    /* If this group is not adjacent to the last one, then we have just
                     * finished a group.  We therefore need to calculate the ranges for the
                     * group we finished, adding them to a HashMap (index by fragment) to be
                     * displayed */
                    // The next meta-group should start with the group we found
                    Self::add_ranges(
                        &mut ranges_by_frag,
                        first_group_in_meta_group,
                        last_group,
                        group_id,
                    );
                    first_group_in_meta_group = group;
                    group_id += 1;
                }
                last_group = group;
            }
            // Make sure that we add the ranges containing the last group
            Self::add_ranges(
                &mut ranges_by_frag,
                first_group_in_meta_group,
                last_group,
                group_id,
            );
        }
        // The final value of group_id is also the number of falseness groups, so return it out
        // of this block along with the ranges we calculated
        (ranges_by_frag, group_id + 1)
    }

    /// A cheeky helper function which adds the ranges between two groups of false rows to
    /// the right places in a HashMap (the map will only ever be `row_groups_by_frag`)
    fn add_ranges(
        ranges_per_frag: &mut HashMap<usize, Vec<FalseRowRange>>,
        start: &[RowLocation],
        end: &[RowLocation],
        group_id: usize,
    ) {
        // Check that we aren't losing information by zipping the two groups
        assert_eq!(start.len(), end.len());
        // Zip through the locations in each group.  Because of their sortedness, we can
        // guaruntee that the pairs are joined up correctly
        for (start_loc, end_loc) in start.iter().zip(end) {
            // Check that each both locations belong to the same group.  This should be
            // guarunteed by the adjacency test, but we test it anyway.
            assert_eq!(start_loc.frag, end_loc.frag);
            // Create a FalseRowGroup, making sure that start <= end (because we could have
            // sets of rows which are false against each other but in the opposite order)
            let false_row_range = FalseRowRange {
                start: start_loc.row.min(end_loc.row),
                end: start_loc.row.max(end_loc.row),
                group: group_id,
            };
            // Insert the newly created group to the HashMap to make sure it's displayed on
            // the correct fragment
            ranges_per_frag
                .entry(start_loc.frag)
                .or_insert_with(|| Vec::with_capacity(0))
                .push(false_row_range);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::RowLocation;

    /// Check that [`RowLocation`]s are sorted by frag index and then row index.  This is required
    /// for the group coalescing to work.
    #[test]
    fn row_loc_ord() {
        /// Helper constructor for [`RowLocation`]s
        fn rl(frag: usize, row: usize) -> RowLocation {
            RowLocation { frag, row }
        }
        assert!(rl(0, 0) < rl(1, 0));
        assert!(rl(0, 1) < rl(1, 0));
        assert!(rl(2, 1) > rl(1, 3));
        assert!(rl(0, 1) < rl(0, 3));
        assert!(rl(1, 0) > rl(0, 100));
    }
}
