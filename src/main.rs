use std::time::Duration;

use speculos_hid::SpeculosHID;
use tokio::time::timeout;

#[tokio::main]
async fn main() {
    let dev = timeout(Duration::from_secs(5 * 60), async {
        loop {
            match SpeculosHID::new("localhost", 8080) {
                Ok(dev) => break dev,
                Err(_) => continue,
            }
        }
    })
    .await
    .expect("unable to create speculos HID device");

    dev.drive().await.expect("unable to drive");
}
