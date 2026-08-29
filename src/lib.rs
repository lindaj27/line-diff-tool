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

/// One `@@ -old_start,old_len +new_start,new_len @@` block: the changed
/// lines plus up to `context` lines of unchanged text on either side.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnifiedHunk<'a> {
    pub old_start: usize,
    pub old_len: usize,
    pub new_start: usize,
    pub new_len: usize,
    pub lines: Vec<Hunk<'a>>,
}

impl<'a> fmt::Display for UnifiedHunk<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(
            f,
            "@@ -{},{} +{},{} @@",
            self.old_start, self.old_len, self.new_start, self.new_len
        )?;
        for (i, line) in self.lines.iter().enumerate() {
            if i > 0 {
                writeln!(f)?;
            }
            write!(f, "{}", line)?;
        }
        Ok(())
    }
}

/// Group a flat edit script into unified-diff hunks, merging two changes
/// into one block whenever the unchanged run between them is short enough
/// that their context windows would overlap.
pub fn unified_hunks<'a>(hunks: &[Hunk<'a>], context: usize) -> Vec<UnifiedHunk<'a>> {
    let change_indices: Vec<usize> = hunks
        .iter()
        .enumerate()
        .filter(|(_, h)| h.op != Op::Equal)
        .map(|(i, _)| i)
        .collect();
    if change_indices.is_empty() {
        return Vec::new();
    }

    let mut groups: Vec<(usize, usize)> = Vec::new();
    let mut group_start = change_indices[0];
    let mut group_end = change_indices[0];
    for &idx in &change_indices[1..] {
        let gap = idx - group_end - 1;
        if gap <= 2 * context {
            group_end = idx;
        } else {
            groups.push((group_start, group_end));
            group_start = idx;
            group_end = idx;
        }
    }
    groups.push((group_start, group_end));

    let mut old_before = vec![0usize; hunks.len() + 1];
    let mut new_before = vec![0usize; hunks.len() + 1];
    for (i, h) in hunks.iter().enumerate() {
        old_before[i + 1] = old_before[i] + if h.op != Op::Insert { 1 } else { 0 };
        new_before[i + 1] = new_before[i] + if h.op != Op::Delete { 1 } else { 0 };
    }

    groups
        .into_iter()
        .map(|(start, end)| {
            let ctx_start = start.saturating_sub(context);
            let ctx_end = (end + context + 1).min(hunks.len());
            let slice = &hunks[ctx_start..ctx_end];

            let old_start_count = old_before[ctx_start];
            let new_start_count = new_before[ctx_start];
            let old_len = old_before[ctx_end] - old_start_count;
            let new_len = new_before[ctx_end] - new_start_count;

            UnifiedHunk {
                old_start: if old_len == 0 {
                    old_start_count
                } else {
                    old_start_count + 1
                },
                old_len,
                new_start: if new_len == 0 {
                    new_start_count
                } else {
                    new_start_count + 1
                },
                new_len,
                lines: slice.to_vec(),
            }
        })
        .collect()
}

/// Render a full unified diff, including `---`/`+++` file headers, from
/// two whole texts.
pub fn format_unified(old_label: &str, new_label: &str, old: &str, new: &str, context: usize) -> String {
    let hunks = diff_text(old, new);
    let groups = unified_hunks(&hunks, context);
    if groups.is_empty() {
        return String::new();
    }

    let mut out = String::new();
    out.push_str(&format!("--- {}\n", old_label));
    out.push_str(&format!("+++ {}\n", new_label));
    for group in groups {
        out.push_str(&group.to_string());
        out.push('\n');
    }
    out
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

    #[test]
    fn unified_hunks_reports_line_ranges_and_context() {
        let old = "a\nb\nc\nd\ne\n";
        let new = "a\nb\nX\nd\ne\n";
        let hunks = diff_text(old, new);
        let groups = unified_hunks(&hunks, 1);
        assert_eq!(groups.len(), 1);
        let g = &groups[0];
        assert_eq!((g.old_start, g.old_len), (2, 3));
        assert_eq!((g.new_start, g.new_len), (2, 3));
    }

    #[test]
    fn unified_hunks_merges_close_changes_and_splits_far_ones() {
        let old = "1\n2\n3\n4\n5\n6\n7\n8\n9\n10\n";
        let new = "X\n2\n3\n4\n5\n6\n7\n8\n9\nY\n";
        let hunks = diff_text(old, new);
        // gap of 8 equal lines with context 1 is far enough apart to split.
        let groups = unified_hunks(&hunks, 1);
        assert_eq!(groups.len(), 2);

        // with enough context the two changes merge into a single hunk.
        let groups = unified_hunks(&hunks, 5);
        assert_eq!(groups.len(), 1);
    }

    #[test]
    fn unified_hunks_empty_for_identical_input() {
        let hunks = diff_text("a\nb\n", "a\nb\n");
        assert!(unified_hunks(&hunks, 3).is_empty());
    }

    #[test]
    fn format_unified_includes_file_headers_and_hunk_header() {
        let out = format_unified("old.txt", "new.txt", "a\nb\nc\n", "a\nX\nc\n", 1);
        assert!(out.starts_with("--- old.txt\n+++ new.txt\n"));
        assert!(out.contains("@@ -1,3 +1,3 @@\n"));
        assert!(out.contains("-b\n+X"));
    }

    #[test]
    fn format_unified_is_empty_for_identical_input() {
        assert_eq!(format_unified("a", "b", "same\n", "same\n", 3), "");
    }
}
