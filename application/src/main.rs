#![no_std]
#![no_main]
#![macro_use]
#![feature(generic_associated_types)]
#![feature(type_alias_impl_trait)]

use drogue_device::bsp::boards::nrf52::microbit::Microbit;
use drogue_device::drivers::ble::gatt::{
    device_info::{DeviceInformationService, DeviceInformationServiceEvent},
    dfu::{FirmwareService, FirmwareServiceEvent},
    temperature::{TemperatureService, TemperatureServiceEvent},
};
use drogue_device::drivers::ble::gatt::{dfu::FirmwareGattService, enable_softdevice};
use drogue_device::firmware::FirmwareManager;
use drogue_device::Board;
use embassy::blocking_mutex::raw::ThreadModeRawMutex;
use embassy::channel::{Channel, DynamicReceiver, DynamicSender};
use embassy::executor::Spawner;
use embassy::time::Ticker;
use embassy::time::{Duration, Timer};
use embassy::util::Forever;
use embassy::util::{select, Either};
use embassy_boot_nrf::updater;
use embassy_nrf::config::Config;
use embassy_nrf::interrupt::Priority;
use embassy_nrf::Peripherals;
use futures::StreamExt;
use heapless::Vec;
use nrf_softdevice::ble::gatt_server;
use nrf_softdevice::{raw, ble::Connection, temperature_celsius, Flash, Softdevice};

#[cfg(feature = "panic-probe")]
use panic_probe as _;

#[cfg(feature = "nrf-softdevice-defmt-rtt")]
use nrf_softdevice_defmt_rtt as _;

#[cfg(feature = "panic-reset")]
use panic_reset as _;

const FIRMWARE_VERSION: &str = env!("CARGO_PKG_VERSION");
const FIRMWARE_REVISION: Option<&str> = option_env!("REVISION");

// Application must run at a lower priority than softdevice
fn config() -> Config {
    let mut config = embassy_nrf::config::Config::default();
    config.gpiote_interrupt_priority = Priority::P2;
    config.time_interrupt_priority = Priority::P2;
    config
}

#[embassy::main(config = "config()")]
async fn main(s: Spawner, p: Peripherals) {
    let board = Microbit::new(p);

    // Spawn the underlying softdevice task
    let sd = enable_softdevice("eclipse-iot");
    s.spawn(softdevice_task(sd)).unwrap();

    let version = FIRMWARE_REVISION.unwrap_or(FIRMWARE_VERSION);
    defmt::info!("Running firmware version {}", version);

    // Watchdog will prevent bootloader from resetting. If your application hangs for more than 5 seconds
    // (depending on bootloader config), it will enter bootloader which may swap the application back.
    s.spawn(watchdog_task()).unwrap();

    // Create a BLE GATT server and make it static
    static GATT: Forever<GattServer> = Forever::new();
    let server = GATT.put(gatt_server::register(sd).unwrap());
    server
        .device_info
        .initialize(b"Eclipse IoT Day", b"1", b"BBC", b"1")
        .unwrap();

    // Fiwmare update service event channel and task
    static EVENTS: Channel<ThreadModeRawMutex, FirmwareServiceEvent, 10> = Channel::new();
    // The updater is the 'application' part of the bootloader that knows where bootloader
    // settings and the firmware update partition is located based on memory.x linker script.
    let dfu = FirmwareManager::new(Flash::take(sd), updater::new());
    let updater = FirmwareGattService::new(&server.firmware, dfu, version.as_bytes(), 64).unwrap();
    s.spawn(updater_task(updater, EVENTS.receiver().into()))
        .unwrap();

    // Starts the bluetooth advertisement and GATT server
    s.spawn(advertiser_task(
        s,
        sd,
        server,
        EVENTS.sender().into(),
        "eclipse-iot",
    ))
    .unwrap();

    // Finally, a blinker application.
    let mut display = board.display;
    loop {
        let _ = display.scroll(version).await;
        Timer::after(Duration::from_secs(5)).await;
    }
}

#[nrf_softdevice::gatt_server]
pub struct GattServer {
    pub firmware: FirmwareService,
    pub temperature: TemperatureService,
    pub device_info: DeviceInformationService,
}

#[embassy::task]
pub async fn updater_task(
    mut dfu: FirmwareGattService<'static, FirmwareManager<Flash>>,
    events: DynamicReceiver<'static, FirmwareServiceEvent>,
) {
    loop {
        let event = events.recv().await;
        if let Err(e) = dfu.handle(&event).await {
            defmt::warn!("Error applying firmware event: {:?}", e);
        }
    }
}

#[embassy::task(pool_size = "4")]
pub async fn gatt_server_task(
    sd: &'static Softdevice,
    conn: Connection,
    server: &'static GattServer,
    events: DynamicSender<'static, FirmwareServiceEvent>,
) {
    let mut notify = false;
    let mut ticker = Ticker::every(Duration::from_secs(1));
    let temp_service = &server.temperature;
    loop {
        let mut interval = None;
        let next = ticker.next();
        match select(
            gatt_server::run(&conn, server, |e| match e {
                GattServerEvent::Temperature(e) => match e {
                    TemperatureServiceEvent::TemperatureCccdWrite { notifications } => {
                        notify = notifications;
                    }
                    TemperatureServiceEvent::PeriodWrite(period) => {
                        interval.replace(Duration::from_millis(period as u64));
                    }
                },
                GattServerEvent::Firmware(e) => {
                    let _ = events.try_send(e);
                }
                _ => {}
            }),
            next,
        )
        .await
        {
            Either::First(res) => {
                if let Err(e) = res {
                    defmt::warn!("gatt_server run exited with error: {:?}", e);
                    return;
                }
            }
            Either::Second(_) => {
                let value: i8 = temperature_celsius(sd).unwrap().to_num();
                defmt::info!("Measured temperature: {}℃", value);

                temp_service.temperature_set(value).unwrap();
                if notify {
                    temp_service.temperature_notify(&conn, value).unwrap();
                }
            }
        }

        if let Some(interval) = interval.take() {
            ticker = Ticker::every(interval);
        }
    }
}

#[embassy::task]
pub async fn advertiser_task(
    spawner: Spawner,
    sd: &'static Softdevice,
    server: &'static GattServer,
    events: DynamicSender<'static, FirmwareServiceEvent>,
    name: &'static str,
) {
    use heapless::Vec;
    use nrf_softdevice::ble::{gatt_server, peripheral};

    let mut adv_data: Vec<u8, 31> = Vec::new();
    #[rustfmt::skip]
        adv_data.extend_from_slice(&[
            0x02, 0x01, raw::BLE_GAP_ADV_FLAGS_LE_ONLY_GENERAL_DISC_MODE as u8,
            0x03, 0x03, 0x00, 0x61,
            (1 + name.len() as u8), 0x09]).unwrap();

    adv_data.extend_from_slice(name.as_bytes()).ok().unwrap();

    #[rustfmt::skip]
        let scan_data = &[
            0x03, 0x03, 0xA, 0x18,
        ];

    loop {
        let config = peripheral::Config::default();
        let adv = peripheral::ConnectableAdvertisement::ScannableUndirected {
            adv_data: &adv_data[..],
            scan_data,
        };
        defmt::debug!("Advertising");
        let conn = peripheral::advertise_connectable(sd, adv, &config)
            .await
            .unwrap();

        defmt::debug!("connection established");
        if let Err(e) = spawner.spawn(gatt_server_task(sd, conn, server, events.clone())) {
            defmt::warn!("Error spawning gatt task");
        }
    }
}

#[embassy::task]
async fn softdevice_task(sd: &'static Softdevice) {
    sd.run().await;
}

// Keeps our system alive
#[embassy::task]
async fn watchdog_task() {
    let mut handle = unsafe { embassy_nrf::wdt::WatchdogHandle::steal(0) };
    loop {
        handle.pet();
        Timer::after(Duration::from_secs(2)).await;
    }
}
