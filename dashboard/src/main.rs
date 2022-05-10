use log::Level;
use patternfly_yew::*;
use yew::prelude::*;
use yew::virtual_dom::VNode;
use yew_oauth2::oauth2::*;
use yew_oauth2::prelude::*;
use yew_router::prelude::*;
use yew_router::router::Render;

mod dashboard;
mod firmware;

use dashboard::Dashboard;
use firmware::Firmware;

pub struct App {}

#[derive(Clone, Switch, PartialEq, Debug)]
enum AppRoute {
    #[to = "/firmware"]
    Firmware,
    #[to = "/"]
    Dashboard,
}

impl Component for App {
    type Message = ();
    type Properties = ();
    fn create(_: &Context<Self>) -> Self {
        Self {}
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        let login = ctx.link().callback_once(|_| {
            OAuth2Dispatcher::<Client>::new().start_login();
        });
        let logout = ctx.link().callback_once(|_| {
            OAuth2Dispatcher::<Client>::new().logout();
        });

        let logo = html_nested! {
            <Logo src="https://www.patternfly.org/assets/images/PF-Masthead-Logo.svg" alt="Patternfly Logo" />
        };

        let tools = vec![{
            let full_name = "Unknown";

            let src = "/assets/images/img_avatar.svg"; //.into();

            // gather items
            let mut items = Vec::<DropdownChildVariant>::new();

            // links
            items.push({
                let mut items = Vec::new();
                items.push(
                    html_nested! {<DropdownItem onclick={logout}>{"Sign Out"}</DropdownItem>},
                );
                (html_nested! {<DropdownItemGroup>{items}</DropdownItemGroup>}).into()
            });

            // render

            let user_toggle = html! {<UserToggle name={full_name} src={src} />};
            html! {
                <>
                <Dropdown
                    id="user-dropdown"
                    plain=true
                    position={Position::Right}
                    toggle_style="display: flex;"
                    toggle={user_toggle}
                    >
                {items}
                </Dropdown>
                </>
            }
        }];

        let unauth_tools = vec![{
            html! {
                <>
                    <div style="padding-right: 8px">
                    <Button label="Log In" variant={Variant::Secondary} onclick={login} />
                    </div>
                    <div>
                    <Button label="Sign Up" variant={Variant::Primary}/>
                    </div>
                </>
            }
        }];

        html! {
            <>
            <OAuth2
                config={
                    Config {
                        client_id: "drogue".into(),
                        auth_url: "https://sso.sandbox.drogue.cloud/auth/realms/drogue/protocol/openid-connect/auth".into(),
                        token_url: "https://sso.sandbox.drogue.cloud/auth/realms/drogue/protocol/openid-connect/token".into(),
                    }
                }
                >
                <Failure><FailureMessage/></Failure>
                <Authenticated>
                    <BackdropViewer/>
                    <ToastViewer/>

                    <Router<AppRoute, ()>
                        //redirect = {Router::redirect(|_|AppRoute::Dashboard)}
                        render = {Self::switch_main(tools)}
                    />
                </Authenticated>
                <NotAuthenticated>
                    <BackdropViewer/>
                    <ToastViewer/>
                    <Page logo={logo} tools={Children::new(unauth_tools)}>
                        <Router<AppRoute>
                            render = { Router::render(move |switch: AppRoute| { match switch {
                                    AppRoute::Dashboard => html!(
                                        <p>{"You need to log in!"}</p>
                                    ),
                                    _ => html!(<LocationRedirect logout_href="/" />),
                            }})}
                        />
                    </Page>
                </NotAuthenticated>
            </OAuth2>

            </>
        }
    }
}

impl App {
    fn switch_main(tools: Vec<VNode>) -> Render<AppRoute, ()> {
        Router::render(move |switch: AppRoute| match switch {
            AppRoute::Dashboard => Self::page(tools.clone(), html! {<Dashboard/>}),
            AppRoute::Firmware => Self::page(tools.clone(), html! {<Firmware/>}),
        })
    }

    fn page(tools: Vec<VNode>, html: Html) -> Html {
        let sidebar = html_nested! {
            <PageSidebar>
                <Nav>
                    <NavRouterItem<AppRoute> to={AppRoute::Dashboard}>{"Dashboard"}</NavRouterItem<AppRoute>>
                    <NavRouterExpandable<AppRoute> title="Manage" expanded=true>
                        <NavRouterItem<AppRoute> to={AppRoute::Firmware}>{"Firmware"}</NavRouterItem<AppRoute>>
                    </NavRouterExpandable<AppRoute>>
                </Nav>
            </PageSidebar>
        };

        let logo = html_nested! {
            <Logo src="https://www.patternfly.org/assets/images/PF-Masthead-Logo.svg" alt="Patternfly Logo" />
        };

        html! {
            <Page
                logo={logo}
                sidebar={sidebar}
                tools={Children::new(tools)}
                >
                { html }
            </Page>
        }
    }
}

fn main() {
    wasm_logger::init(wasm_logger::Config::new(Level::Debug));
    yew::start_app::<App>();
}
