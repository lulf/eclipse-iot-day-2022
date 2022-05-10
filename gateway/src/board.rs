use anyhow::anyhow;
use bluer::{
    gatt::remote::{Characteristic, Service},
    Adapter, Address, Device,
};
use core::pin::Pin;
use futures::{Stream, StreamExt};
use serde_json::json;
use std::str::FromStr;
use std::sync::Arc;
use tokio::time::{sleep, Duration};

pub struct Microbit {
    adapter: Arc<Adapter>,
    device: Address,
    board: Option<Device>,
}

const BOARD_SERVICE_UUID: uuid::Uuid = uuid::Uuid::from_u128(0x0000181A00001000800000805f9b34fb);
const TEMPERATURE_CHAR_UUID: uuid::Uuid = uuid::Uuid::from_u128(0x00002a1f00001000800000805f9b34fb);
const INTERVAL_CHAR_UUID: uuid::Uuid = uuid::Uuid::from_u128(0x00002a2100001000800000805f9b34fb);

unsafe impl Send for Microbit {}

impl Microbit {
    pub fn new(device: &str, adapter: Arc<Adapter>) -> Self {
        Self {
            device: Address::from_str(device).unwrap(),
            adapter,
            board: None,
        }
    }

    async fn connect(&mut self) -> bluer::Result<&mut Device> {
        if self.board.is_none() {
            loop {
                if let Ok(device) = self.adapter.device(self.device) {
                    // Make sure we get a fresh start
                    let _ = device.disconnect().await;
                    sleep(Duration::from_secs(2)).await;
                    match device.is_connected().await {
                        Ok(false) => {
                            log::debug!("Connecting...");
                            loop {
                                match device.connect().await {
                                    Ok(()) => break,
                                    Err(err) => {
                                        log::info!("Connect error: {}", &err);
                                    }
                                }
                            }
                            log::debug!("Connected1");
                            self.board.replace(device);
                            break;
                        }
                        Ok(true) => {
                            log::debug!("Connected2");
                            self.board.replace(device);
                            break;
                        }
                        Err(e) => {
                            log::info!("Error checking connection, retrying: {:?}", e);
                        }
                    }
                }
                sleep(Duration::from_secs(2)).await;
            }
        }
        Ok(self.board.as_mut().unwrap())
    }

    pub async fn set_interval(&mut self, i: u8) -> bluer::Result<()> {
        self.write_char(BOARD_SERVICE_UUID, INTERVAL_CHAR_UUID, &i.to_le_bytes())
            .await
    }

    fn data_to_json(data: &[u8]) -> serde_json::Value {
        let temp: i16 = i16::from_le_bytes([data[0], data[1]]);
        json!({ "temperature": temp })
    }

    pub async fn stream_sensors(
        &mut self,
    ) -> Result<Pin<Box<impl Stream<Item = serde_json::Value>>>, anyhow::Error> {
        let sensors = self
            .stream_char(BOARD_SERVICE_UUID, TEMPERATURE_CHAR_UUID)
            .await?
            .map(|data| Self::data_to_json(&data));

        Ok(Box::pin(sensors))
    }

    async fn write_char(
        &mut self,
        service: uuid::Uuid,
        c: uuid::Uuid,
        value: &[u8],
    ) -> bluer::Result<()> {
        let service = self.find_service(service).await?.unwrap();
        let c = self.find_char(&service, c).await?.unwrap();

        c.write(value).await
    }

    async fn stream_char(
        &mut self,
        service: uuid::Uuid,
        c: uuid::Uuid,
    ) -> Result<impl Stream<Item = Vec<u8>>, anyhow::Error> {
        if let Some(service) = self.find_service(service).await? {
            if let Some(c) = self.find_char(&service, c).await? {
                return Ok(c.notify().await?);
            }
        }
        Err(anyhow!("Error locating service {} and char {}", service, c))
    }

    async fn find_char(
        &mut self,
        service: &Service,
        characteristic: uuid::Uuid,
    ) -> bluer::Result<Option<Characteristic>> {
        for c in service.characteristics().await? {
            let uuid = c.uuid().await?;
            if uuid == characteristic {
                return Ok(Some(c));
            }
        }
        Ok(None)
    }

    async fn find_service(&mut self, service: uuid::Uuid) -> bluer::Result<Option<Service>> {
        let device = self.connect().await?;
        for s in device.services().await? {
            let uuid = s.uuid().await?;
            if uuid == service {
                return Ok(Some(s));
            }
        }
        Ok(None)
    }
}
