use std::io::Write;
use std::path::{Path, PathBuf};

pub(crate) fn prompt(message: String) -> Result<bool, Box<dyn std::error::Error>> {
    let mut stdout = std::io::stdout();

    write!(stdout, "{}\n[y/n] ", message)?;
    stdout.flush()?;

    let mut guess = String::new();

    std::io::stdin().read_line(&mut guess)?;

    let guess = guess.trim();

    if guess.is_empty() {
        Ok(false)
    } else {
        let guess = guess.to_lowercase();
        if guess == "y" || guess == "yes" {
            Ok(true)
        } else {
            Ok(false)
        }
    }
}

pub(crate) fn dir_exist(target: String) -> Result<(), String> {
    let target = expand_tilde(target.as_str());

    if !target.exists() {
        return Err(format!("{}: No such file or directory", target.display()));
    }

    Ok(())
}

pub(crate) fn expand_tilde(s: &str) -> PathBuf {
    if s.starts_with('~') {
        return PathBuf::from(s.replace('~', env!("HOME")));
    }

    PathBuf::from(s)
}

pub(crate) fn is_plain_text(p: &Path) -> bool {
    match tree_magic_mini::from_filepath(p) {
        Some(mime) => {
            if mime == "text/plain" {
                return true;
            }
            false
        }
        None => false,
    }
}
