use std::time::Duration;

use clap::Parser;
use speculos_hid::SpeculosHID;

#[derive(Parser)]
struct Args {
    #[arg(short, value_parser = humantime::parse_duration, default_value = "60s")]
    timeout: Duration,
}

#[tokio::main]
async fn main() {
    let args = Args::parse();

    let dev = tokio::time::timeout(args.timeout, async {
        loop {
            match SpeculosHID::new("localhost", 8080) {
                Ok(dev) => break dev.timeout(args.timeout),
                Err(_) => continue,
            }
        }
    })
    .await
    .expect("unable to create speculos HID device");

    dev.drive().await.expect("unable to drive");
}
