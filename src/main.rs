mod cli;
mod params;
mod solidifier;
mod strings;

use anyhow::Result;

use cli::get_params;
use solidifier::solidify;

fn main() -> Result<()> {
    solidify(&get_params()?)
}
