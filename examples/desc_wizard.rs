use anyhow::{Result, bail};
use bdk::bitcoin::{self, util::bip32::ExtendedPubKey};
use dialoguer::{theme::ColorfulTheme, Confirm, Input, Select};
use slip132::FromSlip132;
use std::str::FromStr;

use signed_address_generator::GenericXpub;

/// This is a sample program to interactively build a descriptor from an xpub / ypub / zpub
fn main() -> Result<()> {
    let theme = ColorfulTheme::default();
    println!("Let's build a descriptor!");
    println!("");

    // STEP 1: network
    println!("What network is this for? DO NOT USE FOR REAL BITCOIN FUNDS PLEASE.");
    let network_selections = &["bitcoin", "testnet", "regtest"];
    let network_choice = Select::with_theme(&theme)
        .with_prompt("Network")
        .default(1)
        .items(&network_selections[..])
        .interact()?;
    println!("");
    let network = bitcoin::Network::from_str(network_selections[network_choice])?;

    // STEP 2: enter your extended public key 
    let extended_public_key: String = Input::with_theme(&theme)
        .with_prompt("Enter your full extended public key with prefix (e.g. xpub123 / ypub123 / zpub123)")
        .interact()?;
    println!("");
    let xpub = ExtendedPubKey::from_slip132_str(&extended_public_key)?;
    if xpub.network != network {
        bail!("This extended public key doesn't match the network you selected.")
    }

    // STEP 3: derivation path
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

    // STEP 4: fingerprint 
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

    // STEP 5: first address as sanity check
    // TODO: Enter 'any' address and we scan the first 10000 for a match
    println!("Enter this wallet's first address to make sure everything is correct");
    let first_address: String = Input::with_theme(&theme)
        .with_prompt("Enter your wallet's first address")
        .interact()?;

    let generic = GenericXpub::new(xpub, derivation_path, fingerprint, first_address, network)?;
    println!("");

    println!("Descriptor:\n{}", generic.descriptor);

    Ok(())
}
