use patternfly_yew::*;
use yew::prelude::*;

pub struct Dashboard {}

impl Component for Dashboard {
    type Message = ();
    type Properties = ();
    fn create(_: &Context<Self>) -> Self {
        Self {}
    }

    fn view(&self, _: &Context<Self>) -> Html {
        html! {
            <>
                <PageSection variant={PageSectionVariant::Light} limit_width=true>
                    <Title level={Level::H1} size={Size::XXXXLarge}>{ "Dashboard" }</Title>
                </PageSection>
                <PageSection>
                    <p>{ "This page will display some overview statistics of how many apps, devices, total quota usage and any important status messages." }</p>
                </PageSection>
            </>
        }
    }
}
