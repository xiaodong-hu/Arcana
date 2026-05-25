mod authority;
mod prompt;
mod record;
mod server;
mod types;

use std::env;
use std::path::PathBuf;
use std::process;

fn main() {
    let args: Vec<String> = env::args().collect();

    // Subcommand: `authority_and_recording auth instruction [project_root]`
    if args.len() >= 3 && args[1] == "auth" && args[2] == "instruction" {
        match prompt::load_or_create_instruction() {
            Ok(content) => print!("{}", content),
            Err(e) => {
                eprintln!("[Arcana] Error: {}", e);
                process::exit(1);
            }
        }
        return;
    }

    // Compatibility subcommand: print the full injected prompt.
    if args.len() >= 3 && args[1] == "auth" && args[2] == "prompt" {
        let root = args
            .get(3)
            .map(PathBuf::from)
            .unwrap_or_else(|| env::current_dir().expect("cannot get cwd"));
        match run_prompt(&root) {
            Ok(content) => print!("{}", content),
            Err(e) => {
                eprintln!("[Arcana] Error: {}", e);
                process::exit(1);
            }
        }
        return;
    }

    if args.len() >= 3 && args[1] == "recovery" {
        let mut root = None;
        let mut list = false;
        let mut to_seq = None;
        let mut confirmed = false;
        let mut idx = 2;
        while idx < args.len() {
            if args[idx] == "--yes" || args[idx] == "-y" {
                confirmed = true;
            } else if args[idx] == "--list" {
                list = true;
            } else if args[idx] == "--to-sequence" {
                idx += 1;
                if idx >= args.len() {
                    eprintln!("[Arcana] Error: --to-sequence requires a number");
                    process::exit(1);
                }
                to_seq = match args[idx].parse::<u64>() {
                    Ok(seq) => Some(seq),
                    Err(e) => {
                        eprintln!("[Arcana] Error: invalid --to-sequence: {}", e);
                        process::exit(1);
                    }
                };
            } else if args[idx].starts_with('-') {
                eprintln!("[Arcana] Error: unknown recovery option `{}`", args[idx]);
                process::exit(1);
            } else if root.is_none() {
                root = Some(PathBuf::from(&args[idx]));
            } else {
                eprintln!(
                    "[Arcana] Error: unexpected recovery argument `{}`",
                    args[idx]
                );
                process::exit(1);
            }
            idx += 1;
        }
        let root = root.unwrap_or_else(|| env::current_dir().expect("cannot get cwd"));
        if list {
            match record::Record::log(&root) {
                Ok(entries) => {
                    for entry in entries.iter().rev() {
                        println!("{}", format_log_entry(entry));
                    }
                }
                Err(e) => {
                    eprintln!("[Arcana] Recovery log failed: {}", e);
                    process::exit(1);
                }
            }
            return;
        }
        if to_seq.is_none() {
            eprintln!("[Arcana] Error: recovery requires `--list` or `--to-sequence <N>`.");
            process::exit(1);
        }
        if !confirmed && !confirm_recovery(&root, to_seq) {
            eprintln!("[Arcana] Recovery aborted.");
            process::exit(1);
        }
        match record::Record::recover(&root, to_seq) {
            Ok(seq) => eprintln!("[Arcana] Recovered {:?} to record seq {}", root, seq),
            Err(e) => {
                eprintln!("[Arcana] Recovery failed: {}", e);
                process::exit(1);
            }
        }
        return;
    }

    // Default: run the server
    let project_root = args
        .get(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| env::current_dir().expect("cannot get cwd"));

    if !project_root.is_dir() {
        eprintln!("[Arcana] Error: {:?} is not a directory", project_root);
        process::exit(1);
    }

    eprintln!(
        "[Arcana] Authority & Record starting for {:?}",
        project_root
    );

    let mut srv = match server::Server::new(project_root) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("[Arcana] Failed to start: {}", e);
            process::exit(1);
        }
    };

    if let Err(e) = srv.run() {
        eprintln!("[Arcana] Server error: {}", e);
        process::exit(1);
    }
}

/// Generate and print the authorized prompt without starting the server.
fn run_prompt(project_root: &PathBuf) -> std::io::Result<String> {
    let auth = authority::Authority::load(project_root.clone())?;
    let content = prompt::generate_prompt(&auth)?;
    // Also write to .arcana/authorized_prompt.md
    let prompt_path = project_root.join(".arcana/authorized_prompt.md");
    std::fs::create_dir_all(prompt_path.parent().unwrap())?;
    std::fs::write(&prompt_path, &content)?;
    Ok(content)
}

fn confirm_recovery(root: &PathBuf, target: Option<u64>) -> bool {
    eprintln!();
    eprintln!("╔════════════════════════════════════════════════════════════════════╗");
    eprintln!("║ WARNING: Arcana recovery will overwrite the working tree.         ║");
    eprintln!("║ Files may be rewritten or removed to match the recorded state.    ║");
    eprintln!("║ The recovery itself is recorded, but unrecorded edits may be lost.║");
    eprintln!("╚════════════════════════════════════════════════════════════════════╝");
    eprintln!("Project: {}", root.display());
    match target {
        Some(seq) => eprintln!("Target record sequence: {seq}"),
        None => eprintln!("Target record sequence: previous sequence"),
    }
    eprint!("Continue recovery? [y/yes to continue]: ");
    let _ = std::io::Write::flush(&mut std::io::stderr());
    let mut input = String::new();
    if std::io::stdin().read_line(&mut input).is_err() {
        return false;
    }
    matches!(input.trim().to_ascii_lowercase().as_str(), "y" | "yes")
}

fn format_log_entry(entry: &record::RecordLogEntry) -> String {
    let head = if entry.is_head { " (HEAD)" } else { "" };
    let target = match (&entry.dst, entry.op.as_str()) {
        (Some(dst), "rename") => format!("{} -> {}", entry.path, dst),
        _ => entry.path.clone(),
    };
    format!(
        "* {:06}{head} {} {} [{}]",
        entry.seq, entry.op, target, entry.ts
    )
}
