use anyhow::Result;
use indicatif::ProgressBar;
use signed_address_generator::{gpg_clearsign, ColdcardJson, Database, Entry, GeneratorState};
use std::{fs, io, io::Write, path::PathBuf, str::FromStr};

use bdk::{
    bitcoin::{self, Address},
    database::MemoryDatabase,
    Wallet,
};
use clap::{Clap, ValueHint};

// TODO: Consider re-ordering to 
    // 1. Ask user to Select Mode (New or Continue)
    //     If New, Ask whether using ColdCard or Generic
    //     If Continue, Jump to 6
    // 2. If New ColdCard, ask user to select colcard-export.json. Load these parameters into memory and go to 4
    // 3. If New Generic, ask user for xpub, derivation path, fingerprint & first address. Load these parameters into memory and go to 4
    // 4. Check that the derived first address matches the expected first address. If yes, go to 5. If no, error message.
    // 5. Ask how many addresses to generate, what index to start from & message to sign -> Generate signed-address-generator.json
    // 6. Load signed-address-generator.json into memory and display key info, ask user to press 'E' to edit a value (e.g. number of addresses) or 'Enter' to proceed
    // 7. Run program to generate addresses, sign them and put them into a database
struct GenericXpub {
    xpub: String,
    derivation_path: String,
    fingerprint: String,
    first_address: String
}

struct App {
    // Step 1
    mode: Mode,
    // Step 2
    path: Option<PathBuf>,
    // Step 3 and also 6
    descriptor: String,
    // TODO print: bool,
}



#[derive(Clap, Clone)]
#[clap(about = r"
Generate addresses from a Coldcard's xpub.
To use this you'll need a coldcard-export.json file.
On your Coldcard go to:
  Advanced > MicroSD Card > Export Wallet > Generic JSON")]
struct Opts {
    // TODO: Consider re-ordering to 
    // 1. Ask user to Select Mode (New or Continue)
    //     If New, Ask whether using ColdCard or Generic
    //     If Continue, Jump to 6
    // 2. If New ColdCard, ask user to select colcard-export.json. Load these parameters into memory and go to 4
    // 3. If New Generic, ask user for xpub, derivation path, fingerprint & first address. Load these parameters into memory and go to 4
    // 4. Check that the derived first address matches the expected first address. If yes, go to 5. If no, error message.
    // 5. Ask how many addresses to generate, what index to start from & message to sign -> Generate signed-address-generator.json
    // 6. Load signed-address-generator.json into memory and display key info, ask user to press 'E' to edit a value (e.g. number of addresses) or 'Enter' to proceed
    // 7. Run program to generate addresses, sign them and put them into a database

    #[clap(name = "PATH/TO/JSON")]
    #[clap(value_hint = ValueHint::FilePath)]
    #[clap(parse(from_os_str))]
    #[clap(
        about = "This can be either a coldcard-export.json if you're starting from scratch. Or a signed-address-generator-state.json if you want to generate additional addresses from a known index."
    )]
    path: PathBuf,
    #[clap(arg_enum)]
    // TODO: explain
    #[clap(about = "TODO explain how to use this")]
    #[clap(long = "mode")]
    mode: Mode,
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

// If you have a coldcard this just does everything automatically.
// If you are using another thing you have to paste in your xpub and derivation path
// it shows you the first address and you confirm that it is correct or that you don't care

#[derive(Clap, Clone, Debug, PartialEq)]
enum Mode {
    // Get xpub and derivation path
    #[clap(name = "new")]
    ImportGeneric,
    // Get coldcard-export.json
    #[clap(name = "coldcard")]
    ImportColdcard,
    // Get signed-address-generator.json
    #[clap(name = "more")]
    DeriveMoreAddresses,
}

fn do_the_work(generator_state: &mut GeneratorState, should_print: bool) -> Result<()> {
    // TODO: should network be a flag?
    let network = bitcoin::Network::Testnet;

    let desc = generator_state.get_descriptor(network)?;

    println!("Descriptor: {}", desc.0);

    let wallet = Wallet::new_offline(desc, None, network, MemoryDatabase::default())?;

        // Skip all these addresses by asking for them
    // TODO: figure out how to just start from an index
    if generator_state.next_index > 0 {
        println!("Skipping addresses...");
        for _i in 0..generator_state.next_index {
            wallet.get_new_address()?;
        }
    }

    // If we skip none, then this is the first address.
    // If we skip N, then N + 1 should be the first
    let first_address = generator_state.check_first_address(&wallet)?;

    if should_print {
        // Now we're actually generating the addresses we care about
        for i in 0..generator_state.number_to_generate {
            let stdout = io::stdout();
            let mut handle = stdout.lock();
            let addy = if i == 0 {
                first_address.clone()
            } else {
                wallet.get_new_address()?
            };

            // pgp sign the address along with our message
            let signed = gpg_clearsign(&addy.to_string(), &generator_state.message)?;
            writeln!(handle, "{}", signed)?;
        }
    } else {
        // Create a new SQLite db file and connect to it
        let db = Database::new()?;

        // Don't want people staring at a blank prompt for minutes
        let pb = ProgressBar::new(generator_state.number_to_generate);

        let mut addresses: Vec<Address> = vec![];

        println!("Generating addresses...");
        for i in 0..generator_state.number_to_generate {
            let addy = if i == 0 {
                first_address.clone()
            } else {
                wallet.get_new_address()?
            };

            addresses.push(addy);
            pb.inc(1);
        }

        dbg!(addresses.clone());

        pb.finish();

        let message_text = &generator_state.message;

        // Would be nice to do this in parallel with rayon but gpg doesn't like that
        println!("PGP signing addresses...");
        let pb = ProgressBar::new(generator_state.number_to_generate);
        for address in addresses {
            let address = address.to_string();
            let signed_message = gpg_clearsign(&address.to_string(), message_text).unwrap();
            pb.inc(1);
            let entry = Entry::new(&address, &signed_message);
            db.insert(entry)?;
        }

        pb.finish();

        generator_state.finish(wallet.get_new_address()?.to_string());
        generator_state.save()?;

        println!(
            "Wrote {} addresses and PGP signed messages to {}",
            generator_state.number_to_generate, db.filename
        );

        println!(
            "Saved this setup to signed-address-generator-state.json.\nNext time use that file and --mode more"
        );
    }

    Ok(())
}

fn main() -> Result<()> {
    let opts: Opts = Opts::parse();

    match opts.mode {
        Mode::ImportGeneric => {
            todo!("We don't support this yet.")
        }
        Mode::ImportColdcard => {
            let wallet_json = fs::read_to_string(opts.path)?;
            let parsed_coldcard = ColdcardJson::from_str(&wallet_json)?;
            let desc = parsed_coldcard.build_descriptor_string();

            // TODO: this only makes sense when we're starting from zero yeah?
            // Regardless of the start index this must be checked
            let next_address = parsed_coldcard.get_first_addresss();

            let mut generator_state = GeneratorState::new(
                desc,
                opts.start_from,
                opts.number_to_generate,
                next_address,
                opts.message,
            );
            do_the_work(&mut generator_state, opts.print)
        }
        Mode::DeriveMoreAddresses => {
            let mut generator_state = GeneratorState::from_path(opts.path)?;
            do_the_work(&mut generator_state, opts.print)
        }
    }
}
