//! Line-based diffing using the Myers O(ND) shortest-edit-script algorithm.
//!
//! The public surface is small on purpose: split text into lines, diff two
//! line slices, get back an ordered list of hunks tagged equal/insert/delete.

use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Op {
    Equal,
    Delete,
    Insert,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Hunk<'a> {
    pub op: Op,
    pub line: &'a str,
}

impl<'a> fmt::Display for Hunk<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let marker = match self.op {
            Op::Equal => ' ',
            Op::Delete => '-',
            Op::Insert => '+',
        };
        write!(f, "{}{}", marker, self.line)
    }
}

/// Split text into lines, dropping a single trailing newline so that
/// "a\nb\n" and "a\nb" both produce ["a", "b"].
pub fn split_lines(text: &str) -> Vec<&str> {
    if text.is_empty() {
        return Vec::new();
    }
    let mut lines: Vec<&str> = text.split('\n').collect();
    if lines.last() == Some(&"") {
        lines.pop();
    }
    lines
}

/// Compute the shortest edit script turning `old` into `new`.
///
/// This is the standard Myers algorithm: a forward search over edit
/// distance `d` that records, for each `d`, the furthest-reaching x on
/// every diagonal `k`, followed by a backtrace through those recorded
/// states to recover the actual edits.
pub fn diff<'a>(old: &[&'a str], new: &[&'a str]) -> Vec<Hunk<'a>> {
    let n = old.len() as isize;
    let m = new.len() as isize;
    let max = n + m;

    if max == 0 {
        return Vec::new();
    }

    let offset = max as usize;
    let width = 2 * max as usize + 1;
    let mut v = vec![0isize; width];
    let mut trace: Vec<Vec<isize>> = Vec::new();
    let mut script_len = 0isize;

    'search: for d in 0..=max {
        trace.push(v.clone());
        let mut k = -d;
        while k <= d {
            let idx = (k + offset as isize) as usize;
            let mut x = if k == -d || (k != d && v[idx - 1] < v[idx + 1]) {
                v[idx + 1]
            } else {
                v[idx - 1] + 1
            };
            let mut y = x - k;
            while x < n && y < m && old[x as usize] == new[y as usize] {
                x += 1;
                y += 1;
            }
            v[idx] = x;
            if x >= n && y >= m {
                script_len = d;
                break 'search;
            }
            k += 2;
        }
    }

    let mut x = n;
    let mut y = m;
    let mut hunks: Vec<Hunk<'a>> = Vec::new();

    for d in (0..=script_len).rev() {
        let v = &trace[d as usize];
        let k = x - y;
        let idx = (k + offset as isize) as usize;
        let go_down = k == -d || (k != d && v[idx - 1] < v[idx + 1]);
        let prev_k = if go_down { k + 1 } else { k - 1 };
        let prev_idx = (prev_k + offset as isize) as usize;
        let prev_x = v[prev_idx];
        let prev_y = prev_x - prev_k;

        while x > prev_x && y > prev_y {
            hunks.push(Hunk {
                op: Op::Equal,
                line: old[(x - 1) as usize],
            });
            x -= 1;
            y -= 1;
        }

        if d > 0 {
            if go_down {
                hunks.push(Hunk {
                    op: Op::Insert,
                    line: new[(y - 1) as usize],
                });
                y -= 1;
            } else {
                hunks.push(Hunk {
                    op: Op::Delete,
                    line: old[(x - 1) as usize],
                });
                x -= 1;
            }
        }
    }

    hunks.reverse();
    hunks
}

/// Convenience wrapper for diffing two whole texts instead of line slices.
pub fn diff_text<'a>(old: &'a str, new: &'a str) -> Vec<Hunk<'a>> {
    diff(&split_lines(old), &split_lines(new))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identical_input_has_no_changes() {
        let hunks = diff_text("a\nb\nc\n", "a\nb\nc\n");
        assert!(hunks.iter().all(|h| h.op == Op::Equal));
        assert_eq!(hunks.len(), 3);
    }

    #[test]
    fn detects_single_insertion() {
        let hunks = diff_text("a\nc\n", "a\nb\nc\n");
        let inserted: Vec<&str> = hunks
            .iter()
            .filter(|h| h.op == Op::Insert)
            .map(|h| h.line)
            .collect();
        assert_eq!(inserted, vec!["b"]);
    }

    #[test]
    fn detects_single_deletion() {
        let hunks = diff_text("a\nb\nc\n", "a\nc\n");
        let deleted: Vec<&str> = hunks
            .iter()
            .filter(|h| h.op == Op::Delete)
            .map(|h| h.line)
            .collect();
        assert_eq!(deleted, vec!["b"]);
    }

    #[test]
    fn empty_inputs_produce_empty_script() {
        assert!(diff_text("", "").is_empty());
    }

    #[test]
    fn one_side_empty_is_all_inserts_or_deletes() {
        let hunks = diff_text("", "a\nb\n");
        assert!(hunks.iter().all(|h| h.op == Op::Insert));
        let hunks = diff_text("a\nb\n", "");
        assert!(hunks.iter().all(|h| h.op == Op::Delete));
    }
}
