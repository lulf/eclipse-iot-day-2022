use patternfly_yew::*;
use yew::prelude::*;

pub struct Firmware {}

impl Component for Firmware {
    type Message = ();
    type Properties = ();
    fn create(_: &Context<Self>) -> Self {
        Self {}
    }

    fn view(&self, _: &Context<Self>) -> Html {
        html! {
            <>
                <PageSection variant={PageSectionVariant::Light} limit_width=true>
                    <Title level={Level::H1} size={Size::XXXXLarge}>{ "Firmware" }</Title>
                </PageSection>
                <PageSection>
                    <p>{ "This view will allow managing firmware builds and a CI/CD-like dashboard of firmware rollouts" }</p>
                </PageSection>
            </>
        }
    }
}
