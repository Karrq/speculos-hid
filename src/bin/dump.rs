use hidapi::{DeviceInfo, HidApi, MAX_REPORT_DESCRIPTOR_SIZE};

const LEDGER_VID: u16 = 0x2c97;
const LEDGER_USAGE_PAGE: u16 = 0xFFA0;

fn is_ledger(dev: &DeviceInfo) -> bool {
    dev.vendor_id() == LEDGER_VID && dev.usage_page() == LEDGER_USAGE_PAGE
}

/// Get a list of ledger devices available
pub fn list_ledgers(api: &HidApi) -> impl Iterator<Item = &DeviceInfo> {
    api.device_list().filter(|dev| is_ledger(dev))
}

fn main() {
    let api = HidApi::new().expect("able to instantiate HID API");
    let dev_info = list_ledgers(&api).next().expect("Ledger found");

    let dev = dev_info
        .open_device(&api)
        .expect("able to open Ledger device HID");

    let mut report_descriptor = [0; MAX_REPORT_DESCRIPTOR_SIZE];
    let report_descriptor_len = dev
        .get_report_descriptor(&mut report_descriptor)
        .expect("able to retrieve Report Descriptor");

    let rd = hex::encode(&report_descriptor[..report_descriptor_len]);

    let prod_id = dev_info.product_id();

    println!("Ledger HID data:\nVendor ID: {LEDGER_VID}\nProduct ID: 0x{prod_id:02X}\nReport descriptor: 0x{rd}");
}
