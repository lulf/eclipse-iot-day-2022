use crate::fetcher::DeviceFetcher;
use drogue_client::{
    core::v1::{ConditionStatus, Conditions},
    dialect,
    openid::AccessTokenProvider,
    registry::v1::Device,
    Section, Translator,
};
use patternfly_yew::*;
use serde::{Deserialize, Serialize};
use yew::prelude::*;
use yew_agent::{Bridge, Bridged};

pub struct Firmware {
    devices: Vec<Device>,
    _fetcher: Box<dyn Bridge<DeviceFetcher>>,
}

#[derive(Serialize, Deserialize, Debug, Copy, Clone)]
pub enum ImagePullPolicy {
    Always,
    IfNotPresent,
}

impl Default for ImagePullPolicy {
    fn default() -> Self {
        Self::IfNotPresent
    }
}

dialect!(FirmwareSpec [Section::Spec => "firmware"]);

#[derive(Serialize, Deserialize, Debug)]
pub enum FirmwareSpec {
    #[serde(rename = "oci")]
    OCI {
        image: String,
        #[serde(rename = "imagePullPolicy", default = "Default::default")]
        image_pull_policy: ImagePullPolicy,
    },
    #[serde(rename = "hawkbit")]
    HAWKBIT { controller: String },
}

dialect!(FirmwareStatus [Section::Status => "firmware"]);

#[derive(Serialize, Deserialize, Debug, Default)]
pub struct FirmwareStatus {
    conditions: Conditions,
    current: String,
    target: String,
}

#[derive(PartialEq, Eq, Debug, Clone)]
pub struct DeviceModel {
    name: String,
    update_type: String,
    conditions: Conditions,
    current: String,
    target: String,
}

impl From<&Device> for DeviceModel {
    fn from(device: &Device) -> Self {
        let spec: FirmwareSpec = device.section::<FirmwareSpec>().unwrap().unwrap();
        let status: FirmwareStatus = device.section::<FirmwareStatus>().unwrap().unwrap();

        Self {
            name: device.metadata.name.clone(),
            update_type: match spec {
                FirmwareSpec::OCI {
                    image: _,
                    image_pull_policy: _,
                } => "OCI".to_string(),
                FirmwareSpec::HAWKBIT { controller: _ } => "Hawkbit".to_string(),
            },
            conditions: status.conditions.clone(),
            current: status.current.clone(),
            target: status.target.clone(),
        }
    }
}

impl DeviceModel {
    fn get_label_state(&self) -> (String, Color) {
        let mut in_sync = false;
        let mut progress = None;
        for condition in self.conditions.0.iter() {
            if condition.r#type == "InSync" && condition.status == "True" {
                in_sync = true;
            } else if condition.r#type == "UpdateProgress" {
                progress = condition.message.clone();
            }
        }
        match (in_sync, progress) {
            (true, _) => ("Synced".to_string(), Color::Green),
            (false, Some(p)) => (format!("Updating ({})", p), Color::Orange),
            (false, _) => ("Unknown".to_string(), Color::Red),
        }
    }
}

impl TableRenderer for DeviceModel {
    fn render(&self, column: ColumnIndex) -> Html {
        let outline = false;
        let (label, color) = self.get_label_state();
        match column.index {
            0 => html! {{&self.name}},
            1 => html! {{&self.update_type}},
            2 => {
                html! {<Label outline={outline} label={format!("{}", &label)} color={color} />}
            }
            3 => html! {{&self.current}},
            4 => html! {{&self.target}},
            _ => html! {},
        }
    }

    fn render_details(&self) -> Vec<Span> {
        vec![Span::max(html! {
            <>
                { "So many details for " }{ &self.name}
            </>
        })]
    }
}

pub enum FirmwareMessage {
    DevicesUpdated(Vec<Device>),
}
impl Component for Firmware {
    type Message = FirmwareMessage;
    type Properties = ();
    fn create(ctx: &Context<Self>) -> Self {
        Self {
            devices: Vec::new(),
            _fetcher: DeviceFetcher::bridge(ctx.link().callback(FirmwareMessage::DevicesUpdated)),
        }
    }

    fn update(&mut self, _ctx: &Context<Self>, msg: Self::Message) -> bool {
        match msg {
            FirmwareMessage::DevicesUpdated(devices) => {
                self.devices = devices;
                true
            }
        }
    }

    fn view(&self, _: &Context<Self>) -> Html {
        let header = html_nested! {
            <TableHeader>
                <TableColumn label="Device" />
                <TableColumn label="Type" />
                <TableColumn label="State" />
                <TableColumn label="Current Version" />
                <TableColumn label="Target Version" />
            </TableHeader>
        };
        let models: Vec<DeviceModel> = self.devices.iter().map(|device| device.into()).collect();
        let model: SharedTableModel<DeviceModel> = models.into();
        html! {
            <>
                <PageSection variant={PageSectionVariant::Light} limit_width=true>
                    <Title level={Level::H1} size={Size::XXXXLarge}>{ "Firmware" }</Title>
                </PageSection>
                <PageSection>
                    <Table<SharedTableModel<DeviceModel>>
                        header={header}
                        entries={model}
                    >

                    </Table<SharedTableModel<DeviceModel>>>
                </PageSection>
            </>
        }
    }
}
