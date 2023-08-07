use std::error::Error;

use speculos_hid::create_device;

fn main() -> Result<(), Box<dyn Error>> {
    let _device = create_device()?;

    Ok(())
}
