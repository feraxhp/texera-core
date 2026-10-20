//! Cursor traversing uniform active intervals (spans) across layered RangeSets.

use std::collections::HashSet;

use super::utils::cmp_pos_side;
use super::utils::FAR;
use super::RangeSetCursor;
use super::RangeValue;
use super::SpanCursor;
use super::RangeSet;


impl<'a, T: RangeValue> SpanCursor<'a, T> {
    /// Creates a new cursor traversing uniform spans across multiple [`RangeSet`]s.
    pub(crate) fn new(
        sets: &[&'a RangeSet<T>],
        skip: Option<&'a HashSet<usize>>,
        min_point: isize,
    ) -> Self {
        Self {
            cursor: RangeSetCursor::from_sets(sets, skip, min_point),
            active: Vec::new(),
            active_to: Vec::new(),
            active_rank: Vec::new(),
            min_active: -1,
            point: None,
            point_from: 0,
            point_rank: 0,
            to: -FAR,
            end_side: 0,
            open_start: -1,
        }
    }

    /// Moves the span cursor to position `pos` with bias `side`.
    pub(crate) fn goto(&mut self, pos: usize, side: i32) {
        self.cursor.goto_with_side(pos, side);
        self.active.clear();
        self.active_to.clear();
        self.active_rank.clear();
        self.min_active = -1;
        self.to = pos as isize;
        self.end_side = side;
        self.open_start = -1;
        self.advance();
    }

    /// Advances active ranges to purge those ending before `(pos, side)`.
    pub(crate) fn forward(&mut self, pos: isize, side: i32) {
        while self.min_active > -1 {
            let idx = self.min_active as usize;
            if cmp_pos_side(
                self.active_to[idx],
                self.active[idx].end_side(),
                pos,
                side,
            ) < 0
            {
                self.remove_active(idx);
            } else {
                break;
            }
        }
        self.cursor.forward(pos, side);
    }

    fn remove_active(&mut self, index: usize) {
        self.active.remove(index);
        self.active_to.remove(index);
        self.active_rank.remove(index);
        self.min_active = find_min_index(&self.active, &self.active_to);
    }

    fn add_active(&mut self, mut track_open: Option<&mut Vec<isize>>) {
        let mut i = 0;
        let value = self.cursor.value.clone().unwrap();
        let to = self.cursor.to;
        let rank = self.cursor.rank;
        while i < self.active_rank.len() {
            let r_diff = (rank as isize) - (self.active_rank[i] as isize);
            let t_diff = to - self.active_to[i];
            if (if r_diff != 0 { r_diff } else { t_diff }) > 0 {
                i += 1;
            } else {
                break;
            }
        }
        self.active.insert(i, value);
        self.active_to.insert(i, to);
        self.active_rank.insert(i, rank);
        if let Some(ref mut track) = track_open {
            track.insert(i, self.cursor.from);
        }
        self.min_active = find_min_index(&self.active, &self.active_to);
    }

    /// Steps forward to the next span boundary or point decoration.
    pub(crate) fn advance(&mut self) {
        let from = self.to;
        let was_point = self.point.is_some();
        self.point = None;
        let mut track_open: Option<Vec<isize>> = match self.open_start < 0 {
            true  => Some(Vec::new()),
            false => None,
        };

        loop {
            let a = self.min_active;
            if a > -1
                && cmp_pos_side(
                    self.active_to[a as usize],
                    self.active[a as usize].end_side(),
                    self.cursor.from,
                    self.cursor.start_side(),
                ) < 0
            {
                let idx = a as usize;
                if self.active_to[idx] > from {
                    self.to = self.active_to[idx];
                    self.end_side = self.active[idx].end_side();
                    break;
                }
                self.remove_active(idx);
                if let Some(ref mut track) = track_open {
                    track.remove(idx);
                }
            } else if self.cursor.value.is_none() {
                self.to = FAR;
                self.end_side = FAR as i32;
                break;
            } else if self.cursor.from > from {
                self.to = self.cursor.from;
                self.end_side = self.cursor.start_side();
                break;
            } else {
                let next_val = self.cursor.value.clone().unwrap();
                if !next_val.point() {
                    self.add_active(track_open.as_mut());
                    self.cursor.advance();
                } else if was_point
                    && self.cursor.to == self.to
                    && self.cursor.from < self.cursor.to
                {
                    self.cursor.advance();
                } else {
                    self.point = Some(next_val.clone());
                    self.point_from = self.cursor.from.max(0) as usize;
                    self.point_rank = self.cursor.rank;
                    self.to = self.cursor.to;
                    self.end_side = next_val.end_side();
                    self.cursor.advance();
                    let to = self.to;
                    let end_side = self.end_side;
                    self.forward(to, end_side);
                    break;
                }
            }
        }

        if let Some(track) = track_open {
            let mut open = 0;
            for &pos in track.iter().rev() {
                if pos < from {
                    open += 1;
                } else {
                    break;
                }
            }
            self.open_start = open;
        }
    }

    /// Collects active range values covering a point ending at `to`.
    pub(crate) fn active_for_point(&self, to: isize) -> Vec<T> {
        if self.active.is_empty() {
            return Vec::new();
        }
        let pt_end_side = self.point.as_ref().map_or(0, |p| p.end_side());
        let mut active = Vec::new();
        for i in (0..self.active.len()).rev() {
            if self.active_rank[i] < self.point_rank {
                break;
            }
            if self.active_to[i] > to
                || (self.active_to[i] == to && self.active[i].end_side() >= pt_end_side)
            {
                active.push(self.active[i].clone());
            }
        }
        active.reverse();
        active
    }

    /// Counts how many active ranges remain open beyond offset `to`.
    pub(crate) fn open_end(&self, to: isize) -> usize {
        let mut open = 0;
        for &act_to in self.active_to.iter().rev() {
            if act_to > to {
                open += 1;
            } else {
                break;
            }
        }
        open
    }
}

fn find_min_index<T: RangeValue>(values: &[T], array: &[isize]) -> isize {
    let mut found: isize = -1;
    let mut found_pos: isize = FAR;
    for i in 0..array.len() {
        let found_side = match found >= 0 {
            true  => values[found as usize].end_side(),
            false => 0,
        };
        if cmp_pos_side(array[i], values[i].end_side(), found_pos, found_side) < 0 {
            found = i as isize;
            found_pos = array[i];
        }
    }
    found
}
