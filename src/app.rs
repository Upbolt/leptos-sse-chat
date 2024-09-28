use leptos::wasm_bindgen::JsCast;
use leptos::{ev::Event, prelude::*};
use leptos_meta::{provide_meta_context, MetaTags, Title};
use leptos_router::{
    components::{Route, Router, Routes},
    StaticSegment,
};
use leptos_use::{use_event_source_with_options, UseEventSourceOptions, UseEventSourceReturn};
use serde::{Deserialize, Serialize};
use web_sys::HtmlInputElement;

pub fn shell(options: LeptosOptions) -> impl IntoView {
    view! {
        <!DOCTYPE html> 
        <html lang="en">
            <head>
                <meta charset="utf-8" />
                <meta name="viewport" content="width=device-width, initial-scale=1" />
                <link rel="stylesheet" id="leptos" href="/pkg/leptos-sse-chat.css" />
                <AutoReload options=options.clone() />
                <HydrationScripts options />
                <MetaTags />
            </head>
            <body>
                <App />
            </body>
        </html>
    }
}

#[component]
pub fn App() -> impl IntoView {
    provide_meta_context();

    view! {
        <Title text="leptos sse chat" />

        <Router>
            <main>
                <Routes fallback=|| "Page not found.".into_view()>
                    <Route path=StaticSegment("") view=HomePage />
                </Routes>
            </main>
        </Router>
    }
}

#[component]
fn HomePage() -> impl IntoView {
    let name = RwSignal::new(String::new());
    let joining = RwSignal::new(false);

    let can_join = AsyncDerived::new(move || async move {
        if !joining.get() {
            return false;
        };

        request_join(name.get()).await.is_ok()
    });

    let can_join = Memo::new(move |_| can_join.get());

    let options = UseEventSourceOptions::default()
        .immediate(false)
        .with_credentials(true);

    let UseEventSourceReturn { data, open, .. } = use_event_source_with_options::<
        ChatEvent,
        JsonSerdeCodec,
    >("http://localhost:3000/chat", options);

    let messages = RwSignal::new(vec![]);

    Effect::new(move |_| {
        if can_join.get().unwrap_or_default() {
            open();
        }
    });

    Effect::new(move |_| {
        let Some(data) = data.get() else {
            return;
        };

        match data {
            ChatEvent::Message(message) => {
                // leptos::logging::log!("{message:?}");
                messages.update(|messages| {
                    messages.push(message);
                });
            }
            ChatEvent::Heartbeat => {}
        };
    });

    view! {
        <Show
            when=move || can_join.get().unwrap_or_default()
            fallback=move || {
                view! {
                    <h1>"register"</h1>

                    <input on:input=move |ev: Event| {
                        let Some(target) = ev.target() else {
                            return;
                        };
                        let Some(input) = target.dyn_ref::<HtmlInputElement>() else {
                            return;
                        };
                        name.set(input.value());
                    } />

                    <button on:click=move |_| joining.set(true)>"join"</button>
                }
            }
        >
            <Chat />
            <ul>
                <For
                    each=move || messages.get().into_iter().enumerate()
                    key=|(index, _)| *index
                    children=move |(_, message)| {
                        view! {
                            <Message
                                author=message.author.clone()
                                content=message.content.clone()
                            />
                        }
                    }
                />
            </ul>
        </Show>
    }
}

#[component]
fn Chat() -> impl IntoView {
    let send_message = ServerAction::<SendMessage>::new();

    view! {
        <ActionForm action=send_message>
            <input type="text" name="message" />
            <input type="submit" value="send" />
        </ActionForm>
    }
}

#[server]
async fn send_message(message: String) -> Result<(), ServerFnError> {
    if message.len() > 100 {
        return Err(ServerFnError::new("message too big"));
    }

    use crate::{event::AppEvent, state::AppState};
    use axum_extra::{headers::Cookie, TypedHeader};
    use http::StatusCode;
    use leptos_axum::{extract, ResponseOptions};

    let TypedHeader::<Cookie>(cookies) = extract().await?;

    let response = expect_context::<ResponseOptions>();
    let state = expect_context::<AppState>();

    let Some(name) = cookies.get("name") else {
        response.set_status(StatusCode::UNAUTHORIZED);
        return Err(ServerFnError::new("unauthorized"));
    };

    println!("send message: {message}");

    state.users.do_send(AppEvent::Message {
        user_name: name.to_string(),
        message,
    });

    Ok(())
}

#[derive(Serialize, Deserialize, Clone, PartialEq)]
enum ChatEvent {
    Message(ChatMessage),
    Heartbeat,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
struct ChatMessage {
    author: String,
    content: String,
}

use codee::string::JsonSerdeCodec;

#[component]
fn Message(author: String, content: String) -> impl IntoView {
    view! { <li>{author}": "{content}</li> }
}

#[server]
async fn request_join(name: String) -> Result<(), ServerFnError> {
    use cookie::Cookie;
    use http::{header::SET_COOKIE, HeaderValue};
    use leptos_axum::ResponseOptions;

    let response = expect_context::<ResponseOptions>();

    let name = Cookie::build(("name", name))
        .path("/")
        .same_site(cookie::SameSite::Lax)
        .http_only(true)
        .build();

    if let Ok(name) = HeaderValue::from_str(&name.to_string()) {
        response.insert_header(SET_COOKIE, name);
    }

    Ok(())
}
