use std::env;
use std::fs;
use std::io::{self, Read};
use std::process;

use tdiff::{diff, split_lines, Op};

/// Read one input source. A path of "-" means stdin; stdin can only be
/// read once, so the result is cached in case both sides ask for it.
fn read_source(path: &str, stdin_cache: &mut Option<String>) -> io::Result<String> {
    if path == "-" {
        if stdin_cache.is_none() {
            let mut buf = String::new();
            io::stdin().read_to_string(&mut buf)?;
            *stdin_cache = Some(buf);
        }
        Ok(stdin_cache.clone().unwrap())
    } else {
        fs::read_to_string(path)
    }
}

fn usage() -> ! {
    eprintln!("usage: tdiff <old> <new>");
    eprintln!("       pass - for either side to read that side from stdin");
    process::exit(2);
}

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();
    if args.len() != 2 {
        usage();
    }

    let mut stdin_cache = None;
    let old_text = read_source(&args[0], &mut stdin_cache).unwrap_or_else(|e| {
        eprintln!("tdiff: {}: {}", args[0], e);
        process::exit(1);
    });
    let new_text = read_source(&args[1], &mut stdin_cache).unwrap_or_else(|e| {
        eprintln!("tdiff: {}: {}", args[1], e);
        process::exit(1);
    });

    let old_lines = split_lines(&old_text);
    let new_lines = split_lines(&new_text);
    let hunks = diff(&old_lines, &new_lines);

    let mut changed = false;
    for hunk in &hunks {
        if hunk.op != Op::Equal {
            changed = true;
        }
        println!("{}", hunk);
    }

    process::exit(if changed { 1 } else { 0 });
}
