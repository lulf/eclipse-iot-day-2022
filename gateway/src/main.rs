use clap::Parser;
use core::str::FromStr;
use futures::future::{select, Either};
use futures::{pin_mut, StreamExt};
use serde_json::json;
use std::sync::Arc;
use std::time::Duration;

mod board;

use crate::board::Microbit;

#[derive(Parser, Debug)]
struct Args {
    #[clap(short, long, parse(from_occurrences))]
    verbose: usize,

    #[clap(short, long)]
    device: String,

    #[clap(short, long, parse(try_from_str=humantime::parse_duration))]
    report_interval: Duration,
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
    let device = args.device;

    let report_interval = args.report_interval.as_secs().min(255).max(1) as u8;

    let session = bluer::Session::new().await?;
    let adapter = Arc::new(session.default_adapter().await?);
    adapter.set_powered(true).await?;
    let address = bluer::Address::from_str(&device)?;

    let discover = adapter.discover_devices().await?;
    pin_mut!(discover);
    while let Some(evt) = discover.next().await {
        log::trace!("Discovery event: {:?}", evt);
        match evt {
            bluer::AdapterEvent::DeviceAdded(a) => {
                if a == address {
                    break;
                }
            }
            _ => {}
        }
    }

    loop {
        let mut board = Microbit::new(&device, adapter.clone());
        board.set_interval(report_interval).await?;
        let s = board.stream_sensors().await?;
        pin_mut!(s);
        let mut view = json!({});
        loop {
            let timeout = tokio::time::sleep(Duration::from_secs(report_interval as u64 + 10));
            let next = s.next();
            pin_mut!(timeout);
            pin_mut!(next);
            match select(next, timeout).await {
                Either::Left((n, _)) => {
                    if let Some(n) = n {
                        merge(&mut view, &n);
                        println!("{}", view);
                    } else {
                        log::info!("Event stream closed, removing device");
                        let _ = adapter.remove_device(address).await;
                        break;
                    }
                }
                Either::Right(_) => {
                    log::info!("Timeout waiting for event, removing device");
                    let _ = adapter.remove_device(address).await;
                    break;
                }
            }
        }
        log::info!("BLE sensor disconnected");
    }
}
