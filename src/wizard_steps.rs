use std::{fs, path::PathBuf, str::FromStr};

use anyhow::{bail, Result};
use bdk::bitcoin::{self, util::bip32::ExtendedPubKey, Address, Network};
use dialoguer::{theme::ColorfulTheme, Confirm, Input, Select};
use slip132::FromSlip132;

use crate::{util, util::build_descriptor, ColdcardJson, Desc, Factory};

pub enum Mode {
    Coldcard,
    Generic,
}

/// Ask whether using ColdCard or Generic
pub fn select_mode() -> Result<Mode> {
    let theme = ColorfulTheme::default();
    println!("Do you have a coldcard-export.json file to import, or do you want to import a generic extended public key?");
    let modes = &["Coldcard", "Generic"];
    let mode_choice = Select::with_theme(&theme)
        .with_prompt("Import type")
        .default(0)
        .items(&modes[..])
        .interact()?;
    println!("");

    if mode_choice == 0 {
        Ok(Mode::Coldcard)
    } else {
        Ok(Mode::Generic)
    }
}

/// Select Network
pub fn select_network() -> Result<Network> {
    let theme = ColorfulTheme::default();

    println!("What network is this for? DO NOT USE FOR REAL BITCOIN FUNDS PLEASE.");
    let network_selections = &["bitcoin", "testnet", "regtest"];
    let network_choice = Select::with_theme(&theme)
        .with_prompt("Network")
        .default(1)
        .items(&network_selections[..])
        .interact()?;
    println!("");
    let network = bitcoin::Network::from_str(network_selections[network_choice])?;

    Ok(network)
}

/// If New ColdCard, ask user to re-run address-factory with coldcard-export.json as an argument.
pub fn new_coldcard_instruction() {
    println!("To use a ColdCard, re-run Address Factory with a coldcard-export.json file as an argument:");
    println!("address-factory PATH/TO/coldcard-export.json");
}

/// Load and parse coldcard-export.json
pub fn new_coldcard_from_file(path: &PathBuf) -> Result<(Desc, Network)> {
    let wallet_json = fs::read_to_string(path)?;
    let parsed_coldcard = ColdcardJson::from_str(&wallet_json)?;
    let desc = parsed_coldcard.build_descriptor_string()?;
    let network = parsed_coldcard.get_network()?;

    // TODO: this only makes sense when we're starting from zero yeah?
    // Regardless of the start index this must be checked
    let next_address = parsed_coldcard.get_first_addresss()?;

    util::check_address(desc.clone(), network, next_address, 0)?;

    Ok((desc, network))
}

/// If New Generic, ask user for xpub, derivation path, fingerprint & first address. Load these parameters into memory and validate
pub fn new_generic(network: Network) -> Result<Desc> {
    let theme = ColorfulTheme::default();

    // STEP 1: enter your extended public key
    println!("Enter your full extended public key with prefix (e.g. xpub123 / ypub123 / zpub123)");
    let extended_public_key: String = Input::with_theme(&theme)
        .with_prompt("Extended public key")
        .interact()?;

    let xpub = ExtendedPubKey::from_slip132_str(&extended_public_key)?;
    if xpub.network != network {
        bail!("This extended public key doesn't match the network you selected.")
    }
    println!("");

    // STEP 2: derivation path
    println!("A derivation looks something like m/84'/1'/0'");
    println!("m / purpose' / coin_type' / account'");
    println!("Here's a nice little overview: https://river.com/learn/terms/d/derivation-path/");
    let derivation_path: String = Input::with_theme(&theme)
        .with_prompt("Enter the derivation path")
        .interact()?;
    // Parse it to check that it's valid
    let derivation_path = bitcoin::util::bip32::DerivationPath::from_str(&derivation_path)?;
    // Count the children to make sure it includes the account
    // TODO: handle weirder wallets where we don't have the account
    if derivation_path.len() != 3 {
        bail!("That derivation path doesn't have the correct length.")
    }
    println!("");

    let descriptor = build_descriptor(xpub, derivation_path)?;

    Ok(descriptor)
}

/// Check that the first address matches
pub fn check_first_address(descriptor: Desc, network: Network) -> Result<Address> {
    let theme = ColorfulTheme::default();
    // TODO: Enter 'any' address and we scan the first 10000 for a match
    println!("Enter this wallet's first address to make sure everything is correct");
    let first_address: String = Input::with_theme(&theme)
        .with_prompt("Enter your wallet's first address")
        .interact()?;

    println!("");
    let address = Address::from_str(&first_address)?;

    util::check_address(descriptor, network, address, 0)
}

/// Ask how many addresses to generate, what index to start from & message to sign
pub fn new_factory(
    descriptor: Desc,
    network: Network,
    next_index: u32,
    number_to_generate: u32,
    config_dir: PathBuf,
) -> Result<Factory> {
    let theme = ColorfulTheme::default();
    println!("How many addresses you want to generate?");
    let number_to_generate: u32 = Input::with_theme(&theme)
        .with_prompt("Number to generate")
        .default(number_to_generate)
        .show_default(true)
        .interact()?;
    println!("");

    println!("How many addresses to skip (because you've used them before)");
    let skip_num: u32 = Input::with_theme(&theme)
        .with_prompt("Number to skip")
        .default(next_index)
        .show_default(true)
        .interact()?;
    println!("");

    println!("Enter a short message to be signed with the address");
    let message: String = Input::with_theme(&theme)
        .with_prompt("Message")
        .interact()?;
    println!("");

    let factory = Factory::new(
        descriptor.to_string(),
        network,
        skip_num,
        number_to_generate,
        message,
        config_dir,
    )?;

    Ok(factory)
}

/// Load address-factory.json into memory and display key info, allow user to edit or proceed
pub fn load_and_edit_factory(path_to_config: PathBuf) -> Result<Factory> {
    let theme = ColorfulTheme::default();

    let mut factory = Factory::from_path(path_to_config.clone())?;

    println!("{}", factory);
    println!("");

    // TODO: if they put in the wrong skip_num I think the index will get screwed up?
    if Confirm::with_theme(&theme)
        .with_prompt("Do you want to make any changes?")
        .interact()?
    {
        factory = new_factory(
            factory.descriptor,
            factory.network,
            factory.next_index,
            factory.number_to_generate,
            path_to_config.parent().unwrap().to_path_buf(),
        )?
    }

    Ok(factory)
}

/// Run program to generate addresses, sign them and put them into a database
pub fn execute(factory: &mut Factory) -> Result<()> {
    factory.generate_addresses()?;
    Ok(())
}
