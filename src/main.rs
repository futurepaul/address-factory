use address_factory::wizard_steps::*;
use anyhow::Result;
use std::path::PathBuf;

use clap::Clap;

#[derive(Clap)]
struct Opts {
    coldcard_json: Option<PathBuf>,
}

fn main() -> Result<()> {
    let opts: Opts = Opts::parse();

    if let Some(path) = opts.coldcard_json {
        let (descriptor, network) = new_coldcard_from_file(&path)?;
        let mut factory = new_factory(descriptor, network, 0, 1000)?;
        execute(&mut factory)
    } else {
        let mode = select_mode()?;

        let mut factory = match mode {
            Mode::New(submode) => match submode {
                SubMode::Coldcard => {
                    new_coldcard_instruction();
                    return Ok(());
                }
                SubMode::Generic => {
                    let network = select_network()?;
                    let descriptor = new_generic(network)?;
                    check_first_address(descriptor.clone(), network)?;
                    new_factory(descriptor, network, 0, 1000)
                }
            },
            Mode::Continue => load_and_edit_factory(),
        }?;

        execute(&mut factory)
    }
}
