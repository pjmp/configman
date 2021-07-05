mod cli;
mod utils;

use std::{error, result};

pub type Error = Box<dyn error::Error>;
pub type Result<T> = result::Result<T, Error>;

fn main() -> Result<()> {
    let app = cli::Configman::new();

    if app.verbose {
        utils::enable_log()?;
    }

    app.run()?;

    Ok(())
}
