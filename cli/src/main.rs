// notd CLI — read/write notd notes from the terminal.
//
// Notes live as YYYY-MM-DD-HHmmss.md files in the storage folder. Lexical
// order == creation order, which is also the dot order in the app. The app
// reconciles .notd-meta.json on next load, so CLI-created notes show up
// automatically.

use std::env;
use std::fs;
use std::io::{self, IsTerminal, Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};

use serde::Deserialize;

const CONFIG_REL: &str = "Library/Application Support/eu.migueldavid.notd/config.json";
const DEFAULT_FOLDER_REL: &str = "Dropbox/Apps/notd";

#[derive(Deserialize)]
struct AppConfig {
    #[serde(rename = "storageFolder")]
    storage_folder: Option<String>,
}

fn usage() {
    let text = concat!(
        "notd — personal drafts app for unformed ideas and todos.\n",
        "Quick capture lands here first; promote the keepers to Obsidian (notes)\n",
        "or Todoist (tasks). Anything left in notd is provisional by design.\n",
        "\n",
        "Usage: notd <command> [args]\n",
        "\n",
        "Commands:\n",
        "  ls                  List notes, oldest → newest (matches app dot order).\n",
        "  show <ref>          Print a note's contents.\n",
        "  path [ref]          Print the storage folder, or a note's full path.\n",
        "  new [body...]       Create a note. Body from args; otherwise stdin.\n",
        "  append <ref> [b...] Append text to a note (a blank line is added first).\n",
        "  edit <ref>          Open a note in $EDITOR.\n",
        "  rm <ref>            Delete a note.\n",
        "  grep <pattern>      Case-insensitive search; each hit is prefixed with\n",
        "                      the note's index, so `rm <idx>` pairs naturally.\n",
        "  folder              Print the active storage folder.\n",
        "  help                Show this message.\n",
        "\n",
        "<ref> is a 1-based index from `ls`, a filename like 2026-05-10-214827.md,\n",
        "or \"last\" for the most recent note.",
    );
    println!("{text}");
}

fn home() -> PathBuf {
    env::var("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("/"))
}

fn config_path() -> PathBuf {
    home().join(CONFIG_REL)
}

fn default_folder() -> PathBuf {
    home().join(DEFAULT_FOLDER_REL)
}

fn get_folder() -> PathBuf {
    if let Ok(raw) = fs::read_to_string(config_path()) {
        if let Ok(cfg) = serde_json::from_str::<AppConfig>(&raw) {
            if let Some(f) = cfg.storage_folder.filter(|s| !s.is_empty()) {
                return PathBuf::from(f);
            }
        }
    }
    default_folder()
}

fn require_folder() -> Result<PathBuf, String> {
    let f = get_folder();
    if !f.is_dir() {
        return Err(format!("notd folder does not exist: {}", f.display()));
    }
    Ok(f)
}

fn list_notes(folder: &Path) -> Vec<String> {
    let mut out = Vec::new();
    let entries = match fs::read_dir(folder) {
        Ok(e) => e,
        Err(_) => return out,
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        if path.extension().and_then(|s| s.to_str()) != Some("md") {
            continue;
        }
        let name = match path.file_name().and_then(|s| s.to_str()) {
            Some(n) => n.to_string(),
            None => continue,
        };
        if name.starts_with('.') {
            continue;
        }
        out.push(name);
    }
    out.sort();
    out
}

fn resolve_ref(folder: &Path, refstr: &str) -> Result<String, String> {
    if refstr.is_empty() {
        return Err("missing <ref>".into());
    }
    if refstr == "last" {
        return list_notes(folder)
            .into_iter()
            .next_back()
            .ok_or_else(|| format!("no notes in {}", folder.display()));
    }
    if refstr.ends_with(".md") {
        if refstr.contains('/') || refstr.contains('\\') || refstr.contains("..") {
            return Err(format!("invalid filename: {refstr}"));
        }
        if !folder.join(refstr).is_file() {
            return Err(format!("note not found: {refstr}"));
        }
        return Ok(refstr.to_string());
    }
    if let Ok(n) = refstr.parse::<usize>() {
        if n == 0 {
            return Err("index must be >= 1".into());
        }
        let notes = list_notes(folder);
        return notes
            .into_iter()
            .nth(n - 1)
            .ok_or_else(|| format!("no note at index {n}"));
    }
    Err(format!(
        "invalid ref: {refstr} (use index, filename.md, or 'last')"
    ))
}

// Builds YYYY-MM-DD-HHmmss.md in local time. macOS-only: shells out to `date`,
// avoiding a chrono/time dep for a one-line need.
fn new_filename() -> Result<String, String> {
    let out = Command::new("date")
        .arg("+%Y-%m-%d-%H%M%S.md")
        .output()
        .map_err(|e| format!("date: {e}"))?;
    if !out.status.success() {
        return Err("date command failed".into());
    }
    let s = String::from_utf8_lossy(&out.stdout).trim().to_string();
    if s.len() != "YYYY-MM-DD-HHmmss.md".len() {
        return Err(format!("unexpected date output: {s:?}"));
    }
    Ok(s)
}

fn resolve_collision(folder: &Path, base: &str) -> String {
    if !folder.join(base).exists() {
        return base.to_string();
    }
    let stem = base.strip_suffix(".md").unwrap_or(base);
    let mut i: u32 = 2;
    loop {
        let cand = format!("{stem}-{i}.md");
        if !folder.join(&cand).exists() {
            return cand;
        }
        i += 1;
    }
}

fn preview_line(path: &Path) -> String {
    let content = match fs::read_to_string(path) {
        Ok(c) => c,
        Err(_) => return String::new(),
    };
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let collapsed: String = trimmed.split_whitespace().collect::<Vec<_>>().join(" ");
        return collapsed.chars().take(60).collect();
    }
    String::new()
}

fn read_stdin() -> io::Result<String> {
    let mut s = String::new();
    io::stdin().read_to_string(&mut s)?;
    Ok(s)
}

fn body_from_args_or_stdin(args: &[String], require: bool) -> Result<Option<String>, String> {
    if !args.is_empty() {
        return Ok(Some(args.join(" ")));
    }
    if !io::stdin().is_terminal() {
        return read_stdin().map(Some).map_err(|e| format!("stdin: {e}"));
    }
    if require {
        return Err("nothing to write (pass args or pipe stdin)".into());
    }
    Ok(None)
}

fn cmd_ls() -> Result<(), String> {
    let folder = require_folder()?;
    let notes = list_notes(&folder);
    if notes.is_empty() {
        println!("(no notes in {})", folder.display());
        return Ok(());
    }
    for (i, name) in notes.iter().enumerate() {
        let preview = preview_line(&folder.join(name));
        if preview.is_empty() {
            println!("{:>3}  {}  (empty)", i + 1, name);
        } else {
            println!("{:>3}  {}  {}", i + 1, name, preview);
        }
    }
    Ok(())
}

fn cmd_show(args: &[String]) -> Result<(), String> {
    let folder = require_folder()?;
    let refstr = args.first().map(String::as_str).unwrap_or("");
    let name = resolve_ref(&folder, refstr)?;
    let bytes = fs::read(folder.join(&name)).map_err(|e| format!("read: {e}"))?;
    io::stdout()
        .write_all(&bytes)
        .map_err(|e| format!("write: {e}"))?;
    Ok(())
}

fn cmd_path(args: &[String]) -> Result<(), String> {
    let folder = require_folder()?;
    if args.is_empty() {
        println!("{}", folder.display());
        return Ok(());
    }
    let name = resolve_ref(&folder, &args[0])?;
    println!("{}/{}", folder.display(), name);
    Ok(())
}

fn cmd_folder() -> Result<(), String> {
    println!("{}", get_folder().display());
    Ok(())
}

fn cmd_new(args: &[String]) -> Result<(), String> {
    let folder = get_folder();
    fs::create_dir_all(&folder).map_err(|e| format!("mkdir: {e}"))?;
    let body = body_from_args_or_stdin(args, false)?.unwrap_or_default();
    let base = new_filename()?;
    let name = resolve_collision(&folder, &base);
    fs::write(folder.join(&name), body.as_bytes()).map_err(|e| format!("write: {e}"))?;
    println!("{name}");
    Ok(())
}

fn cmd_append(args: &[String]) -> Result<(), String> {
    let folder = require_folder()?;
    let refstr = args.first().map(String::as_str).unwrap_or("");
    let name = resolve_ref(&folder, refstr)?;
    let rest: &[String] = if args.len() > 1 { &args[1..] } else { &[] };
    let body = body_from_args_or_stdin(rest, true)?.unwrap();

    let path = folder.join(&name);
    let existing = fs::read_to_string(&path).unwrap_or_default();
    let combined = if existing.is_empty() {
        body
    } else {
        let sep = if existing.ends_with('\n') {
            "\n"
        } else {
            "\n\n"
        };
        format!("{existing}{sep}{body}")
    };
    fs::write(&path, combined.as_bytes()).map_err(|e| format!("write: {e}"))?;
    println!("{name}");
    Ok(())
}

fn cmd_edit(args: &[String]) -> Result<(), String> {
    let folder = require_folder()?;
    let refstr = args.first().map(String::as_str).unwrap_or("");
    let name = resolve_ref(&folder, refstr)?;
    let editor = env::var("EDITOR").unwrap_or_else(|_| "vi".to_string());
    let status = Command::new(&editor)
        .arg(folder.join(&name))
        .status()
        .map_err(|e| format!("spawn {editor}: {e}"))?;
    if !status.success() {
        return Err(format!(
            "{editor} exited with {}",
            status.code().unwrap_or(-1)
        ));
    }
    Ok(())
}

fn cmd_rm(args: &[String]) -> Result<(), String> {
    let folder = require_folder()?;
    let refstr = args.first().map(String::as_str).unwrap_or("");
    let name = resolve_ref(&folder, refstr)?;
    fs::remove_file(folder.join(&name)).map_err(|e| format!("rm: {e}"))?;
    println!("removed {name}");
    Ok(())
}

// Native case-insensitive substring search. Output mirrors `ls`: each hit
// line starts with the note's 1-based index so it pairs with `notd rm <n>`.
fn cmd_grep(args: &[String]) -> Result<bool, String> {
    let folder = require_folder()?;
    let pattern = match args.first() {
        Some(p) => p,
        None => return Err("missing pattern".into()),
    };
    let needle = pattern.to_lowercase();
    let notes = list_notes(&folder);
    let mut any = false;
    for (i, name) in notes.iter().enumerate() {
        let content = match fs::read_to_string(folder.join(name)) {
            Ok(c) => c,
            Err(_) => continue,
        };
        for (lineno, line) in content.lines().enumerate() {
            if line.to_lowercase().contains(&needle) {
                println!("{:>3}  {}:{}: {}", i + 1, name, lineno + 1, line);
                any = true;
            }
        }
    }
    Ok(any)
}

// Restore the default SIGPIPE handler so piping into `head`, `less`, etc.
// terminates this process cleanly instead of panicking inside println!.
#[cfg(unix)]
fn reset_sigpipe() {
    // SAFETY: signal() is a thread-safe POSIX call; we're setting the handler
    // before any threads or pipes exist.
    unsafe {
        libc::signal(libc::SIGPIPE, libc::SIG_DFL);
    }
}

#[cfg(not(unix))]
fn reset_sigpipe() {}

fn main() -> ExitCode {
    reset_sigpipe();
    let args: Vec<String> = env::args().collect();
    let sub = args.get(1).map(String::as_str).unwrap_or("help");
    let rest: Vec<String> = args.iter().skip(2).cloned().collect();

    let result = match sub {
        "ls" | "list" => cmd_ls(),
        "show" | "cat" => cmd_show(&rest),
        "path" => cmd_path(&rest),
        "folder" => cmd_folder(),
        "new" | "add" => cmd_new(&rest),
        "append" => cmd_append(&rest),
        "edit" => cmd_edit(&rest),
        "rm" | "del" | "delete" => cmd_rm(&rest),
        "grep" | "search" => match cmd_grep(&rest) {
            Ok(true) => Ok(()),
            Ok(false) => return ExitCode::from(1),
            Err(e) => Err(e),
        },
        "help" | "-h" | "--help" => {
            usage();
            Ok(())
        }
        other => {
            eprintln!("Unknown command: {other}");
            usage();
            return ExitCode::from(2);
        }
    };

    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("error: {e}");
            ExitCode::from(1)
        }
    }
}
