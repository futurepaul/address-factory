use std::{fs, str::FromStr};

use anyhow::{bail, Result};
use bdk::bitcoin::{self, util::bip32::ExtendedPubKey, Address, Network};
use dialoguer::{theme::ColorfulTheme, Confirm, Input, Select};
use slip132::FromSlip132;

use crate::{ColdcardJson, Desc, Factory, util, util::build_descriptor};

pub enum Mode {
    New(SubMode),
    Continue,
}

pub enum SubMode {
    Coldcard,
    Generic,
}

/// 1. Ask user to Select Mode (New or Continue)
///     If New, Ask whether using ColdCard or Generic
///     If Continue, Jump to 6
pub fn select_mode() -> Result<Mode> {
    let theme = ColorfulTheme::default();
    println!("Do you want to create a New address factory or Continue an existing one?");
    let modes = &["New", "Continue"];
    let mode_choice = Select::with_theme(&theme)
        .with_prompt("Mode")
        .default(0)
        .items(&modes[..])
        .interact()?;
    println!("");

    // If "New"
    if mode_choice == 0 {
        println!("Do you have a coldcard-export.json file to import, or do you want to import a generic extended public key?");
        let submodes = &["Coldcard", "Generic"];
        let submode_choice = Select::with_theme(&theme)
            .with_prompt("Import type")
            .default(0)
            .items(&submodes[..])
            .interact()?;
        println!("");

        if submode_choice == 0 {
            Ok(Mode::New(SubMode::Coldcard))
        } else {
            Ok(Mode::New(SubMode::Generic))
        }

    // Otherwise "Continue"
    } else {
        Ok(Mode::Continue)
    }
}

/// 1a. Select Network
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

/// 2. If New ColdCard, ask user to select colcard-export.json. Load these parameters into memory and go to 4
pub fn new_coldcard(network: Network) -> Result<Desc> {
    let theme = ColorfulTheme::default();
    let path: String = Input::with_theme(&theme)
        .with_prompt("PATH/TO/coldcard-export.json")
        .interact()?;

    let wallet_json = fs::read_to_string(path)?;
    let parsed_coldcard = ColdcardJson::from_str(&wallet_json)?;
    let desc = parsed_coldcard.build_descriptor_string()?;

    // TODO: this only makes sense when we're starting from zero yeah?
    // Regardless of the start index this must be checked
    let next_address = parsed_coldcard.get_first_addresss()?;

    let address = util::check_address(desc.clone(), network, next_address, 0)?;

    Ok(desc)
}

/// 3. If New Generic, ask user for xpub, derivation path, fingerprint & first address. Load these parameters into memory and validate
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

    // STEP 3: fingerprint
    // TODO: is it okay for this to be optional?
    let has_fingerprint = Confirm::with_theme(&theme)
        .with_prompt("Do you know your wallet's parent fingerprint?")
        .default(true)
        .interact()?;

    let fingerprint = if has_fingerprint {
        println!("Some information about fingerprints...");
        let fingerprint: String = Input::with_theme(&theme)
            .with_prompt("Enter your wallet fingerprint")
            .interact()?;
        fingerprint
    } else {
        let fingy = xpub.parent_fingerprint.to_string();
        fingy
    };
    println!("");

    let descriptor = build_descriptor(xpub, derivation_path, &fingerprint)?;

    Ok(descriptor)
}

/// 4. Check that the first address matches
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

/// 5. Ask how many addresses to generate, what index to start from & message to sign -> Generate address-factory.json
pub fn new_factory(descriptor: Desc, network: Network) -> Result<Factory> {
    let theme = ColorfulTheme::default();
    println!("How many addresses you want to generate?");
    let number_to_generate: u64 = Input::with_theme(&theme)
        .with_prompt("Number to generate")
        .interact()?;
    println!("");

    println!("How many addresses to skip (because you've used them before)");
    let skip_num: u64 = Input::with_theme(&theme)
        .with_prompt("Number to skip")
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
    )?;

    Ok(factory)
}

/// 6. Load address-factory.json into memory and display key info, 
/// allow user to edit (jumps to 5) or proceed
pub fn load_and_edit_factory() -> Result<Factory> {
    let theme = ColorfulTheme::default();
    let path: String = Input::with_theme(&theme)
        .with_prompt("PATH/TO/address-factory.json")
        .interact()?;

    let mut factory = Factory::from_path(path.into())?;

    println!("{:?}", factory);
    println!("");

    // TODO: if they put in the wrong skip_num I think the index will get screwed up?
    if Confirm::with_theme(&theme)
        .with_prompt("Do you want to make any changes?")
        .interact()?
    {
        factory = new_factory(factory.descriptor()?, factory.network)?
    }

    Ok(factory)
}

/// 7. Run program to generate addresses, sign them and put them into a database
pub fn execute(factory: &mut Factory) -> Result<()> {
    factory.generate_addresses()?;
    Ok(())
}
