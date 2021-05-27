mod dotty;
mod utils;

use structopt::StructOpt;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // if true {
    //     return Ok(());
    // }

    dotty::Dotty::from_args().run()?;

    Ok(())
}
