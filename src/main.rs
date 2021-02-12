use anyhow::Result;
use indicatif::ProgressBar;
use signed_address_generator::{gpg_clearsign, ColdcardJson, Database};
use std::{
    fs::{self, File},
    io,
    io::{BufWriter, Write},
    path::PathBuf,
    str::FromStr,
};

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
    #[clap(short = 'p', long = "print")]
    #[clap(about = "Print the signed addresses instead of storing them in an SQLite db.")]
    print: bool,
}

fn main() -> Result<()> {
    let opts: Opts = Opts::parse();

    let wallet_json = fs::read_to_string(opts.path)?;

    let parsed_coldcard = ColdcardJson::from_str(&wallet_json)?;

    let desc = parsed_coldcard.get_descriptor(bitcoin::Network::Testnet);

    println!("Descriptor: {}", desc.0);

    let wallet = Wallet::new_offline(
        desc,
        None,
        bitcoin::Network::Testnet,
        MemoryDatabase::default(),
    )?;

    // Skip all these addresses by asking for them
    // TODO: figure out how to just start from an index
    for _i in 0..opts.start_from {
        wallet.get_new_address()?;
    }

    if opts.print {
        // Now we're actually generating the addresses we care about
        for _i in 0..opts.number_to_generate {
            let stdout = io::stdout();
            let mut handle = stdout.lock();
            let addy = wallet.get_new_address()?;

            // pgp sign the address along with our message
            let signed = gpg_clearsign(&addy.to_string(), &opts.message)?;
            writeln!(handle, "{}", signed)?;
        }
    } else {
        // Create a new SQLite db file and connect to it
        let db = Database::new()?;

        // Don't want people staring at a blank prompt for minutes
        let pb = ProgressBar::new(opts.number_to_generate);

        for i in 0..opts.number_to_generate {
            let addy = wallet.get_new_address()?.to_string();

            let signed = gpg_clearsign(&addy, &opts.message)?;

            db.insert(&addy, &signed)?;

            pb.set_message(&format!("generating #{}", i + 1));
            pb.inc(1);
        }

        pb.finish_with_message("done generating");
    }

    Ok(())
}
