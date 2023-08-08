use std::{
    fs::File,
    io::{Cursor, Write},
    time::Duration,
};

use anyhow::{Context, Result};
use byteorder::{BigEndian, ReadBytesExt, WriteBytesExt};
use isahc::{prelude::Configurable, AsyncReadResponseExt, Request, RequestExt};
use tokio::sync::mpsc;

use uhid_virt::{Bus, CreateParams, OutputEvent, UHIDDevice};

const LEDGER_VID: u32 = 0x2c97;
const LEDGER_PID: u32 = 0x5011;

const LEDGER_HID_CHANNEL: u16 = 0x0101;
const LEDGER_HID_TAG: u8 = 0x05;
const LEDGER_PACKET_SIZE: usize = 64;

fn create_device() -> Result<UHIDDevice<File>> {
    //dumped from real device
    let rd_data =
        hex::decode("06a0ff0901a1010903150026ff007508954081080904150026ff00750895409108c0")
            .unwrap();

    let params = CreateParams {
        name: "speculos".to_string(),
        phys: "".to_string(),
        uniq: "".to_string(),
        bus: Bus::USB,
        vendor: LEDGER_VID,
        product: LEDGER_PID,
        version: 0,
        country: 0,
        rd_data,
    };

    UHIDDevice::create(params).context("creating UHIDDevice")
}

pub struct SpeculosHID {
    addr: String,
    device: UHIDDevice<File>,
    timeout: Duration,
}

impl SpeculosHID {
    pub fn new(host: &str, port: u16) -> Result<Self> {
        let device = create_device().context("creating device")?;

        //verify address is correct
        std::net::TcpStream::connect((host, port)).context("checking speculos connection")?;
        let addr = format!("http://{host}:{port}");

        Ok(Self {
            addr,
            device,
            timeout: Duration::from_secs(60),
        })
    }

    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    /// Read an incoming message from the inner `UHIDDevice`
    ///
    /// Will take care of removing the HID framing data
    fn read_apdu(&mut self) -> Option<Vec<u8>> {
        let mut expected_apdu_len = 0;
        let mut sequence_idx = 0;
        let mut apdu = Vec::with_capacity(260); //MAX APDU message length

        loop {
            //read from device
            let output = match self.device.read() {
                Ok(output) => output,
                Err(e) => {
                    eprint!("errored on reading from device: ");
                    match e {
                        uhid_virt::StreamError::Io(io) => eprintln!("{io:?}"),
                        uhid_virt::StreamError::UnknownEventType(ev) => {
                            eprintln!("unknown event type: {ev}")
                        }
                    }
                    break None;
                }
            };

            match &output {
                OutputEvent::Start { .. } => eprintln!("UHID Start"),
                OutputEvent::Stop => eprintln!("UHID Stop"),
                OutputEvent::Open => eprintln!("UHID Open"),
                OutputEvent::Close => eprintln!("UHID Close"),
                OutputEvent::Output { .. } => {}
                OutputEvent::GetReport { .. } => eprintln!("UHID GetReport"),
                OutputEvent::SetReport { .. } => eprintln!("UHID SetReport"),
            }

            let OutputEvent::Output { data } = output else {
                break None;
            };
            println!("hid => {}", hex::encode(&data));

            //Verify channel, tag
            let mut reader = Cursor::new(&data);

            let chan = reader.read_u16::<BigEndian>().ok()?;
            let tag = reader.read_u8().ok()?;
            let seq_idx = reader.read_u16::<BigEndian>().ok()?;

            if chan != LEDGER_HID_CHANNEL || tag != LEDGER_HID_TAG {
                eprintln!("Wront channel (got {chan}) or tag (got {tag})");
                break None;
            }

            //check sequence index
            // each loop it's increased to match the expected incoming message
            if seq_idx != sequence_idx {
                eprintln!("wrong sequence idx. got {seq_idx}, expected {sequence_idx}");
                break None;
            }

            if seq_idx == 0 {
                expected_apdu_len = reader.read_u16::<BigEndian>().ok()? as usize;
            }

            let available = data.len() - reader.position() as usize;
            let needed = expected_apdu_len - apdu.len();
            let payload_end = std::cmp::min(available, needed);

            let payload = &data[reader.position() as usize..][..payload_end];
            apdu.extend_from_slice(payload);

            if apdu.len() >= expected_apdu_len {
                break Some(apdu);
            }
            sequence_idx += 1;
        }
    }

    /// Write an outgoing message to the inner `UHIDDevice`
    ///
    /// Will take care of framing the data according to HID
    fn write_apdu(&mut self, mut data: Vec<u8>) -> Result<()> {
        let apdu_len = data.len();

        let mut in_data = Vec::with_capacity(2 + apdu_len);
        in_data.write_u16::<BigEndian>(apdu_len as u16).unwrap();
        in_data.append(&mut data);

        for (sequence_idx, chunk) in in_data.chunks(LEDGER_PACKET_SIZE - 5).enumerate() {
            let mut buffer = Vec::with_capacity(LEDGER_PACKET_SIZE);

            buffer.write_u16::<BigEndian>(LEDGER_HID_CHANNEL).unwrap();
            buffer.write_u8(LEDGER_HID_TAG).unwrap();
            buffer.write_u16::<BigEndian>(sequence_idx as u16).unwrap();
            buffer.write_all(chunk).unwrap();

            println!("hid <= {}", hex::encode(&buffer));
            self.device
                .write(&buffer)
                .context("writing to UHID device")?;
        }

        Ok(())
    }

    fn spawn_emulator_loop(
        &self,
    ) -> (
        mpsc::UnboundedSender<Vec<u8>>,
        mpsc::UnboundedReceiver<Vec<u8>>,
    ) {
        let (tx_emu, mut rx) = mpsc::unbounded_channel::<Vec<u8>>();
        let (tx, rx_hid) = mpsc::unbounded_channel::<Vec<u8>>();

        let endpoint = format!("{}/apdu", self.addr);
        let timeout = self.timeout;
        tokio::spawn(async move {
            //Received a message from the HID interface
            while let Some(msg) = rx.recv().await {
                let msg = hex::encode(msg);
                let msg = json::object! {
                    "data":  msg,
                };

                // send as POST to /apdu
                let req = Request::post(&endpoint)
                    .timeout(timeout)
                    .body(json::stringify(msg))
                    .context("building apdu request")
                    .unwrap();

                let mut resp = req
                    .send_async()
                    .await
                    .context("sending apdu request")
                    .unwrap();

                let resp = resp
                    .text()
                    .await
                    .context("reading response body as text")
                    .unwrap();

                let json = json::parse(&resp)
                    .context("parsing response as json")
                    .unwrap();
                let data = json["data"]
                    .as_str()
                    .context("retrieving response 'data'")
                    .unwrap();

                let body = hex::decode(data).context("decoding data as hex").unwrap();

                //send reply `body` back to HID interface
                tx.send(body).expect("able to send back to HID interface");
            }
            eprintln!("rx channel closed")
        });

        (tx_emu, rx_hid)
    }

    /// This is a blocking future
    pub async fn drive(mut self) -> Result<()> {
        let (tx, mut rx) = self.spawn_emulator_loop();

        loop {
            let Some(data) = self.read_apdu() else {
                continue;
            };

            println!("apdu => {}", hex::encode(&data));

            // send to emulator
            tx.send(data).expect("able to send to emulator");

            // wait for response
            let Some(data) = rx.recv().await else {
                eprintln!("no more messages for HID device from emulator");
                break;
            };

            println!("apdu <= {}", hex::encode(&data));

            //write to device
            if let Err(e) = self.write_apdu(data) {
                eprintln!("device write returned error: {e:?}");
                break;
            }
        }

        Ok(())
    }
}
