use anyhow::Result;
use signed_address_generator::wizard_steps::*;

fn main() -> Result<()> {
    let mode = select_mode()?;

    let mut factory = match mode {
        Mode::New(submode) => match submode {
            SubMode::Coldcard => {
                let network = select_network()?;
                let descriptor = new_coldcard(network)?;
                new_factory(descriptor, network)
            }
            SubMode::Generic => {
                let network = select_network()?;
                let descriptor = new_generic(network)?;
                check_first_address(descriptor.clone(), network)?;
                new_factory(descriptor, network)
            }
        },
        Mode::Continue => load_and_edit_factory(),
    }?;

    execute(&mut factory)
}
