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
tdiff [-u] <old> <new>
```

Either argument can be a file path or `-` to read that side from stdin:

```sh
tdiff old.txt new.txt
cat new.txt | tdiff old.txt -
some-command | tdiff baseline.txt -
```

By default, output uses one line per row: a leading space for unchanged
lines, `-` for lines only in the old input, `+` for lines only in the new
one. Pass `-u` / `--unified` for the more familiar unified diff format
instead, with `---`/`+++` file headers and `@@ -l,s +l,s @@` hunk headers
carrying three lines of context:

```sh
tdiff -u old.txt new.txt
```

```
--- old.txt
+++ new.txt
@@ -1,3 +1,3 @@
 one
-two
+two and a half
 three
```

Exit status follows the `diff(1)` convention: 0 if the inputs are
identical, 1 if they differ, 2 on a usage error.

## Status

Early. Line-level diffing and unified diff output work; the context size
for `-u` is fixed at 3 lines (no `-U` flag yet), and there's no word-level
mode. See the library tests in `src/lib.rs` for the cases currently
covered.

## License

MIT, see LICENSE.
