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

pub struct Firmware {
    devices: Vec<Device>,
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
    state: String,
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
            state: "Unknown".to_string(),
            current: status.current.clone(),
            target: status.target.clone(),
        }
    }
}

impl TableRenderer for DeviceModel {
    fn render(&self, column: ColumnIndex) -> Html {
        match column.index {
            0 => html! {{&self.name}},
            1 => html! {{&self.update_type}},
            2 => html! {{&self.state}},
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
    DevicesUpdate(Vec<Device>),
}
impl Component for Firmware {
    type Message = FirmwareMessage;
    type Properties = ();
    fn create(_: &Context<Self>) -> Self {
        Self {
            devices: Vec::new(),
        }
    }

    fn update(&mut self, _ctx: &Context<Self>, msg: Self::Message) -> bool {
        match msg {
            FirmwareMessage::DevicesUpdate(devices) => {
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
