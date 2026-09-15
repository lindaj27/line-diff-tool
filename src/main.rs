use std::env;
use std::fs;
use std::io::{self, Read};
use std::process;

use tdiff::{diff, format_unified, split_lines, Op};

const DEFAULT_CONTEXT: usize = 3;

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
    eprintln!("usage: tdiff [-u] [-U<n>] <old> <new>");
    eprintln!("       pass - for either side to read that side from stdin");
    eprintln!("       -u, --unified   print unified diff with @@ hunk headers");
    eprintln!("       -U<n>           lines of context around each change (implies -u, default 3)");
    process::exit(2);
}

/// Parse a `-U<n>` argument, either attached ("-U5") or as a separate
/// following argument ("-U" "5"). Returns the context width and how many
/// of the remaining args (0 or 1) it consumed beyond the flag itself.
fn parse_context_flag(flag: &str, rest: &[String]) -> (usize, usize) {
    let attached = &flag[2..];
    if !attached.is_empty() {
        let n = attached.parse().unwrap_or_else(|_| {
            eprintln!("tdiff: invalid context value: {}", attached);
            process::exit(2);
        });
        (n, 0)
    } else {
        let arg = rest.first().unwrap_or_else(|| usage());
        let n = arg.parse().unwrap_or_else(|_| {
            eprintln!("tdiff: invalid context value: {}", arg);
            process::exit(2);
        });
        (n, 1)
    }
}

fn main() {
    let mut unified = false;
    let mut context = DEFAULT_CONTEXT;
    let mut paths: Vec<String> = Vec::new();
    let args: Vec<String> = env::args().skip(1).collect();
    let mut i = 0;
    while i < args.len() {
        let arg = &args[i];
        match arg.as_str() {
            "-u" | "--unified" => unified = true,
            _ if arg.starts_with("-U") => {
                unified = true;
                let (n, consumed) = parse_context_flag(arg, &args[i + 1..]);
                context = n;
                i += consumed;
            }
            _ => paths.push(arg.clone()),
        }
        i += 1;
    }
    if paths.len() != 2 {
        usage();
    }

    let mut stdin_cache = None;
    let old_text = read_source(&paths[0], &mut stdin_cache).unwrap_or_else(|e| {
        eprintln!("tdiff: {}: {}", paths[0], e);
        process::exit(1);
    });
    let new_text = read_source(&paths[1], &mut stdin_cache).unwrap_or_else(|e| {
        eprintln!("tdiff: {}: {}", paths[1], e);
        process::exit(1);
    });

    if unified {
        let out = format_unified(&paths[0], &paths[1], &old_text, &new_text, context);
        print!("{}", out);
        process::exit(if out.is_empty() { 0 } else { 1 });
    }

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
