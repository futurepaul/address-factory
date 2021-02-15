use ::bitcoin::util::bip32::ExtendedPubKey;
use anyhow::Result;
use bdk::bitcoin;
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
    let network: bitcoin::Network = bitcoin::Network::from_str(network_selections[network_choice])?;

    // STEP 2: enter your xpub
    let xpub: String = Input::with_theme(&theme)
        .with_prompt("Enter your xpub / ypub / zpub")
        .interact()?;
    println!("");
    let actual_xpub = ExtendedPubKey::from_slip132_str(&xpub)?;
    let xpub = actual_xpub.to_string();
    let xpub_network = actual_xpub.network;

    // STEP 3: derivation path
    println!("A derivation looks something like m/84'/1'/0'");
    println!("m / purpose' / coin_type' / account'");
    println!("Here's a nice little overview: https://river.com/learn/terms/d/derivation-path/");
    let derivation_path: String = Input::with_theme(&theme)
        .with_prompt("Enter the derivation path")
        .interact()?;
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
        let fingy = actual_xpub.parent_fingerprint.to_string();
        dbg!(fingy.clone());
        fingy
    };
    println!("");

    // STEP 5: first address as sanity check
    println!("Enter this wallet's first address to make sure everything is correct");
    let first_address: String = Input::with_theme(&theme)
        .with_prompt("Enter your wallet's first address")
        .interact()?;

    let generic = GenericXpub::new(xpub, derivation_path, fingerprint, first_address, network)?;
    println!("");

    println!("Descriptor:\n{}", generic.descriptor);

    Ok(())
}
