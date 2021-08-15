//! Code for maintaining and navigating an undo history.

use std::collections::VecDeque;

use crate::{full::FullComp, spec::CompSpec};

/// An undo history of the composition being edited by Jigsaw.
#[derive(Debug, Clone)]
pub struct History {
    /// The sequence of [`CompSpec`]s representing the most recent undo history.  This is ordered
    /// chronologically with the most recent edit at the end.
    history: VecDeque<CompSpec>,
    /// The index within `history` of the [`CompSpec`] being currently displayed.  Redo and undo
    /// corresponds to incrementing/decrementing this pointer, respectively.
    current_undo_index: usize,
    /// A [`FullComp`] which stores the same data as `self.history[self.current_undo_index]`
    full_comp: FullComp,
}

impl History {
    /// Creates a new [`History`] containing only one [`CompSpec`]
    pub fn new(spec: CompSpec) -> Self {
        let full_comp = FullComp::from_spec(&spec);
        let mut history = VecDeque::new();
        history.push_back(spec);
        Self {
            history,
            current_undo_index: 0,
            full_comp,
        }
    }

    /// Moves one step backwards in the undo history.  Returns `false` if we are already on the
    /// oldest undo step.
    pub fn undo(&mut self) -> bool {
        if self.current_undo_index == 0 {
            false
        } else {
            self.current_undo_index -= 1;
            true
        }
    }

    /// Moves one step forwards in the undo history.  Returns `false` if we are already on the
    /// most recent undo step.
    pub fn redo(&mut self) -> bool {
        if self.current_undo_index == self.history.len() - 1 {
            false
        } else {
            self.current_undo_index += 1;
            true
        }
    }

    pub fn comp_spec(&self) -> &CompSpec {
        &self.history[self.current_undo_index]
    }

    pub fn full_comp(&self) -> &FullComp {
        &self.full_comp
    }
}
