use clap::Parser;
use drgdfu::{FirmwareDevice, GattBoard};
use futures::lock::Mutex;
use futures::{pin_mut, StreamExt};
use serde_json::json;
use std::process::exit;
use std::sync::Arc;
use std::time::{Duration, Instant};

mod board;

use crate::board::Microbit;

#[derive(Parser, Debug)]
struct Args {
    #[clap(short, long, parse(from_occurrences))]
    verbose: usize,

    #[clap(short, long, parse(try_from_str=humantime::parse_duration))]
    timeout: Option<Duration>,

    #[clap(short, long)]
    device: String,

    #[clap(short, long)]
    report_interval: Option<u8>,
}

fn merge(a: &mut serde_json::Value, b: &serde_json::Value) {
    match (a, b) {
        (&mut serde_json::Value::Object(ref mut a), &serde_json::Value::Object(ref b)) => {
            for (k, v) in b {
                merge(a.entry(k.clone()).or_insert(serde_json::Value::Null), v);
            }
        }
        (a, b) => {
            *a = b.clone();
        }
    }
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    stderrlog::new().verbosity(args.verbose).init().unwrap();

    let session = bluer::Session::new().await?;
    let adapter = Arc::new(session.default_adapter().await?);
    adapter.set_powered(true).await?;

    let last_event = Arc::new(Mutex::new(Instant::now()));

    if let Some(timeout) = args.timeout {
        let last_event = last_event.clone();

        tokio::task::spawn(async move {
            loop {
                tokio::time::sleep(Duration::from_secs(1)).await;

                if Instant::now() - *last_event.lock().await > timeout {
                    log::error!("Reached timeout ({timeout:?}) with no events, exiting ...");
                    exit(1);
                }
            }
        });
    }

    // Run device discovery
    let discover = adapter.discover_devices().await?;
    tokio::task::spawn(async move {
        pin_mut!(discover);
        while let Some(evt) = discover.next().await {
            log::info!("Discovery event: {:?}", evt);
        }
    });

    let device = args.device;
    let report_interval = args.report_interval;
    let version = {
        let mut gatt = GattBoard::new(&device, adapter.clone());
        let version = gatt.version().await?;
        version
    };
    println!("Connected to board! Running version {}", version);
    let mut board = Microbit::new(&device, adapter.clone());
    if let Some(i) = report_interval {
        board.set_interval(i).await?;
        return Ok(());
    }

    let s = board.stream_sensors().await?;
    pin_mut!(s);
    let mut view = json!({});
    while let Some(n) = s.next().await {
        *last_event.lock().await = Instant::now();
        let previous = view.clone();
        merge(&mut view, &n);
        if previous != view {
            println!("{}", view);
        }
    }
    log::info!("BLE sensor disconnected, shutting down");
    Ok(())
}
