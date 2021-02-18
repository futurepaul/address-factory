use anyhow::Result;
use signed_address_generator::wizard_steps::*;

fn main() -> Result<()> {
    let mode = select_mode()?;

    let mut factory = match mode {
        Mode::New(submode) => match submode {
            SubMode::Coldcard => {
                let network = select_network()?;
                let (descriptor, address) = new_coldcard(network.clone())?;
                new_factory(descriptor, network, address)
            }
            SubMode::Generic => {
                let network = select_network()?;
                let descriptor = new_generic(network.clone())?;
                let address = validate_descriptor(descriptor.clone(), network)?;
                new_factory(descriptor, network, address)
            }
        },
        Mode::Continue => load_and_edit_factory(),
    }?;

    execute(&mut factory)
}
