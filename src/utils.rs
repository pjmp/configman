use std::path::PathBuf;

pub fn enable_log() -> crate::Result<()> {
    use fmtlog::{formats::SIMPLE2, Config};

    fmtlog::new(Config::new().format(SIMPLE2)).set()?;

    Ok(())
}

pub(crate) fn prompt(message: String) -> bool {
    // use std::io::Write;
    // let mut stdout = std::io::stdout();

    // write!(stdout, "{}\n[y/n] ", message)?;
    // stdout.flush()?;

    // let mut guess = String::new();

    // std::io::stdin().read_line(&mut guess)?;

    // let guess = guess.trim();

    // if guess.is_empty() {
    //     Ok(false)
    // } else {
    //     let guess = guess.to_lowercase();
    //     if guess == "y" || guess == "yes" {
    //         Ok(true)
    //     } else {
    //         Ok(false)
    //     }
    // }

    use dialoguer::{theme::ColorfulTheme, Confirm};

    Confirm::with_theme(&ColorfulTheme::default())
        // .with_prompt("Do you want to continue?")
        .with_prompt(message)
        .interact()
        .unwrap_or(false)
}

pub(crate) fn validate_path(target: &str) -> Result<(), String> {
    expand(target)
        .canonicalize()
        .map_err(|e| e.to_string())
        .map(|_| Ok(()))?
}

pub(crate) fn expand(s: &str) -> PathBuf {
    let path = if s.contains('~') {
        PathBuf::from(s.replace('~', env!("HOME")))
    } else {
        PathBuf::from(s)
    };

    match path.canonicalize() {
        Ok(p) => p,
        // ignoring errors as `validate_path` will handle it
        Err(_) => path,
    }
}
