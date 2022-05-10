use drogue_client::{
    core::v1::{ConditionStatus, Conditions},
    dialect,
    openid::AccessTokenProvider,
    registry::v1::Device,
    Section, Translator,
};
use gloo::{
    console::{self, Timer},
    timers::callback::{Interval, Timeout},
};
use std::collections::HashSet;
use std::time::Duration;
use yew_agent::{Agent, AgentLink, Context, HandlerId};

pub type DrogueClient = drogue_client::registry::v1::Client;

pub struct DeviceFetcher {
    client: DrogueClient,
    link: AgentLink<DeviceFetcher>,
    subscribers: HashSet<HandlerId>,
    interval: Interval,
}

pub enum FetcherMessage {
    Fetch,
}

impl Agent for DeviceFetcher {
    type Reach = Context<Self>;
    type Message = FetcherMessage;
    type Input = ();
    type Output = Vec<Device>;

    fn create(link: AgentLink<Self>) -> Self {
        let token = "drg_2SJYjt_NaYMyI3GRuEBuGPQWTLDmSOWBh49Ui3QO6po";
        let tp = AccessTokenProvider {
            user: "lulf".to_string(),
            token: token.to_string(),
        };
        log::info!("Starting agent");

        let url = reqwest::Url::parse("https://api.sandbox.drogue.cloud").unwrap();
        let client = DrogueClient::new(reqwest::Client::new(), url, tp);

        let interval = {
            let link = link.clone();
            Interval::new(1000, move || link.send_message(FetcherMessage::Fetch))
        };

        Self {
            client,
            link,
            subscribers: HashSet::new(),
            interval,
        }
    }

    fn update(&mut self, msg: Self::Message) {
        log::info!("Agent update called!");
        let client = self.client.clone();
        let subscribers = self.subscribers.clone();
        let link = self.link.clone();
        match msg {
            FetcherMessage::Fetch => {
                wasm_bindgen_futures::spawn_local(async move {
                    let devices = client
                        .list_devices("eclipse-iot-day", None)
                        .await
                        .unwrap()
                        .unwrap();
                    for sub in subscribers.iter() {
                        link.respond(*sub, devices.clone());
                    }
                });
            }
        }
    }

    fn handle_input(&mut self, _: Self::Input, _id: HandlerId) {}

    fn connected(&mut self, id: HandlerId) {
        self.subscribers.insert(id);
    }

    fn disconnected(&mut self, id: HandlerId) {
        self.subscribers.remove(&id);
    }
}
