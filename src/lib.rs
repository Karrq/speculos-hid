use std::{fs::File, io};

use uhid_virt::{Bus, CreateParams, UHIDDevice};

const LEDGER_VID: u32 = 0x2c97;

pub fn create_device() -> io::Result<UHIDDevice<File>> {
    let params = CreateParams {
        name: "speculos".to_string(),
        phys: "".to_string(),
        uniq: "".to_string(),
        bus: Bus::USB,
        vendor: LEDGER_VID,
        product: todo!(),
        version: 0,
        country: 0,
        rd_data: todo!(),
    };

    UHIDDevice::create(params)
}
