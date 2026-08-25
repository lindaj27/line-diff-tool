# tdiff

A small library for computing line-level diffs between two pieces of text,
plus a CLI wrapper around it.

I wanted something I could pipe command output through to see what changed
between two runs, without shelling out to system `diff` and parsing its
output format. `tdiff` computes the diff itself using the Myers shortest-edit
-script algorithm and prints a plain +/-/space format.

## Library

```rust
use tdiff::diff_text;

let old = "one\ntwo\nthree\n";
let new = "one\ntwo and a half\nthree\n";

for hunk in diff_text(old, new) {
    println!("{}", hunk);
}
```

```
 one
-two
+two and a half
 three
```

The core type is `Hunk`, a line paired with an `Op` of `Equal`, `Delete`, or
`Insert`. `diff` works on `&[&str]` if you've already split your input into
lines some other way; `diff_text` and `split_lines` are there for the common
case of starting from whole strings.

## CLI

```
tdiff <old> <new>
```

Either argument can be a file path or `-` to read that side from stdin:

```sh
tdiff old.txt new.txt
cat new.txt | tdiff old.txt -
some-command | tdiff baseline.txt -
```

Output uses one line per row: a leading space for unchanged lines, `-` for
lines only in the old input, `+` for lines only in the new one. Exit status
follows the `diff(1)` convention: 0 if the inputs are identical, 1 if they
differ, 2 on a usage error.

## Status

Early. Line-level diffing works; there's no unified-diff-style output with
hunk headers yet, and no word-level mode. See the library tests in
`src/lib.rs` for the cases currently covered.

## License

MIT, see LICENSE.
