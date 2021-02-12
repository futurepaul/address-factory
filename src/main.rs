use anyhow::Result;
use indicatif::ProgressBar;
use signed_address_generator::{ColdcardJson, gpg_clearsign};
use std::{fs::{self, File}, io, io::{BufWriter, Write}, path::PathBuf, str::FromStr};

use bdk::{bitcoin, database::MemoryDatabase, Wallet};
use clap::{Clap, ValueHint};

#[derive(Clap)]
#[clap(about = r"
Generate addresses from a Coldcard's xpub.
To use this you'll need a coldcard-export.json file.
On your Coldcard go to:
  Advanced > MicroSD Card > Export Wallet > Generic JSON")]
struct Opts {
    #[clap(name = "PATH/TO/coldcard-export.json")]
    #[clap(value_hint = ValueHint::FilePath)]
    #[clap(parse(from_os_str))]
    #[clap(about = "This file is exported by your Coldcard")]
    path: PathBuf,
    #[clap(short = 'n', long = "number", default_value = "100")]
    #[clap(about = "The number of addresses you want to generate")]
    number_to_generate: u64,
    #[clap(short = 'f', long = "from", default_value = "0")]
    #[clap(about = "The number of addresses to skip (because you've used them before)")]
    start_from: u64,
    #[clap(short = 'm', long = "message")]
    #[clap(default_value = "This is a donation address for me, Satoshi Nakamoto:")]
    #[clap(about = "A short message to sign along with the address")]
    message: String,
    #[clap(name = "OUTPUT", short = 'o', long = "output", value_hint = ValueHint::FilePath)]
    #[clap(parse(from_os_str))]
    #[clap(
        about = "Where you'd like to save the generated addresses.\nBy default they're just printed."
    )]
    output: Option<PathBuf>,
}

fn main() -> Result<()> {
    let opts: Opts = Opts::parse();

    let wallet_json = fs::read_to_string(opts.path)?;

    let parsed_coldcard = ColdcardJson::from_str(&wallet_json)?;

    let desc = parsed_coldcard.get_descriptor(bitcoin::Network::Testnet);

    println!("{}", desc.0);

    let wallet = Wallet::new_offline(
        desc,
        None,
        bitcoin::Network::Testnet,
        MemoryDatabase::default(),
    )?;

    match opts.output {
        // If we get a file to write to, put the addresses there
        Some(path) => {
            // From the std docs:
            // > This function will create a file if it does not exist,
            // > and will truncate it if it does.
            let mut file = File::create(path)?;

            let pb = ProgressBar::new(opts.start_from);

            // Skip all these addresses by asking for them
            // TODO: figure out how to just start from an index
            for i in 0..opts.start_from {
                wallet.get_new_address()?;

                // This takes non-zero time so here's a progress bar
                pb.set_message(&format!("skipping #{}", i + 1));
                pb.inc(1);
            }

            pb.finish_with_message("done skipping");

            let pb = ProgressBar::new(opts.number_to_generate);

            // Now we're actually generating the addresses we care about
            for i in 0..opts.number_to_generate {
                let addy = wallet.get_new_address()?;

                // pgp sign the address along with our message
                let signed = gpg_clearsign(&addy.to_string(), &opts.message)?;
                writeln!(file, "{}", signed)?; 

                pb.set_message(&format!("generating #{}", i + 1));
                pb.inc(1);
            }

            pb.finish_with_message("done generating");
        }
        // Otherwise just print to stdout
        // TODO: there's probably a more generic way to do this
        // so we don't have to duplicate code
        None => {
            for _i in 0..opts.start_from {
                wallet.get_new_address()?;
            }

            for _i in 0..opts.number_to_generate {
                let stdout = io::stdout();
                let mut handle = stdout.lock();
                let addy = wallet.get_new_address()?;

                // pgp sign the address along with our message
                let signed = gpg_clearsign(&addy.to_string(), &opts.message)?;
                writeln!(handle, "{}", signed)?; 
            }
        }
    }

    Ok(())
}
