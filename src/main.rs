use address_factory::wizard_steps::*;
use anyhow::Result;
use std::path::PathBuf;

use clap::Clap;

use directories::ProjectDirs;

#[derive(Clap)]
#[clap(version = "0.1 Alpha", author = "Paul M. <paul@paul.lol>")]
struct Opts {
    #[clap(long)]
    purge: bool,
    coldcard_json: Option<PathBuf>,
}
// The basic logic:
//
// CONTINUE EXISTING SETUP
// IF an address-factory.json exists where we expect it:
// 1. load the config
// 2. load_and_edit_factory
// 3. execute
//
// OTHERWISE WE'RE DOING A NEW SETUP
// IF run with coldcard path
// 1. parse file
// 2. new_factory
// 3. execute
// Otherwise ask if they want to use coldcard or import generic
// IF coldcard, tell them to re-run with coldcard json path
// ELSE do generic setup
// 1. new_factory
// 2. execute

fn main() -> Result<()> {
    // Look up the appropriate config dir for the system
    let project = ProjectDirs::from("com", "futurepaul", "Address Factory").unwrap();
    let config_dir = project.config_dir();

    // Check for an existing address-factory.json in the config dir
    let path_to_config = config_dir.join("address-factory.json");

    let opts: Opts = Opts::parse();

    if opts.purge {
        purge(config_dir)?;
        return Ok(());
    }

    // Create our factory object from all sorts of scenarios
    let mut factory = if path_to_config.exists() {
        println!(
            "Found an address-factory.json file in {}",
            path_to_config.to_string_lossy()
        );
        println!("We'll use that.");

        // Load config as factory
        load_and_edit_factory(path_to_config)?
    } else {
        // If user supplied a coldcard-export.json we'll use that
        if let Some(path) = opts.coldcard_json {
            let (descriptor, network) = new_coldcard_from_file(&path)?;
            new_factory(descriptor, network, 0, 1000, config_dir.to_path_buf())?
        } else {
            // TODO: handle importing an address-factory.json from a non-standard location
            println!(
                "Didn't find an existing Address Factory configuration. Let's create a new one."
            );

            // Pick Coldcard or Generic
            let mode = select_mode()?;
            match mode {
                // If Coldcard we exit with an instruction to supply a path
                Mode::Coldcard => {
                    new_coldcard_instruction();
                    std::process::exit(0);
                }
                // Otherwise we build a descriptor and create a factory from it
                Mode::Generic => {
                    let network = select_network()?;
                    let descriptor = new_generic(network)?;
                    check_first_address(descriptor.clone(), network)?;
                    new_factory(descriptor, network, 0, 1000, config_dir.to_path_buf())?
                }
            }
        }
    };

    execute(&mut factory)
}
