use leptos::prelude::*;
use leptos_meta::{provide_meta_context, MetaTags, Title};
use leptos_router::{
    components::{Route, Router, Routes},
    StaticSegment,
};
use leptos_use::{use_event_source_with_options, UseEventSourceOptions, UseEventSourceReturn};
use web_sys::js_sys::RegExp;
use web_sys::Url;

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
    let request_join = ServerAction::<RequestJoin>::new();
    let can_join = Signal::derive(move || {
        request_join
            .value()
            .get()
            .and_then(|join| join.ok())
            .is_some()
    });

    let options = UseEventSourceOptions::default()
        .immediate(false)
        .with_credentials(true);

    let UseEventSourceReturn { data, open, .. } =
        use_event_source_with_options::<ChatEvent, JsonSerdeCodec>("/chat", options);

    let messages = RwSignal::new(vec![]);

    Effect::new(move |_| {
        if can_join.get() {
            open();
        }
    });

    Effect::new(move |_| {
        let Some(data) = data.get() else {
            return;
        };

        match data {
            ChatEvent::Message(message) => {
                messages.update(|messages| {
                    messages.push(message);
                });
            }
            ChatEvent::Heartbeat => {}
        };
    });

    view! {
        <Show
            when=move || can_join.get()
            fallback=move || {
                view! {
                    <h1>"register"</h1>

                    <ActionForm action=request_join>
                        <input type="text" name="name" />
                        <input type="submit" value="join" />
                    </ActionForm>
                }
            }
        >
            <Chat />
            <ul>
                <For
                    each=move || messages.get()
                    key=|message| message.id.clone()
                    children=move |message| {
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

    state.users.do_send(AppEvent::Message {
        user_name: name.to_string(),
        message,
    });

    Ok(())
}

use codee::string::JsonSerdeCodec;

use leptos::either::Either;

use crate::app_event::ChatEvent;

#[component]
fn Message(author: String, content: String) -> impl IntoView {
    let spotify_track_id = Url::new(&content).ok().and_then(|url| {
        let pathname = url.pathname();

        let valid_track_id = RegExp::new("[a-zA-Z0-9]", "").test(&pathname["/track/".len()..]);

        (url.origin() == "https://open.spotify.com"
            && pathname.starts_with("/track/")
            && valid_track_id)
            .then_some(pathname["/track/".len()..].to_string())
    });

    if let Some(spotify_track_id) = spotify_track_id {
        Either::Left(view! { <SpotifyEmbed track_id=spotify_track_id /> })
    } else {
        Either::Right(view! { <li>{author}": "{content}</li> })
    }
}

#[component]
fn SpotifyEmbed(track_id: String) -> impl IntoView {
    view! {
        <div>
            <SpotifyEmbedInner track_id=track_id attr:loading="lazy" />
        </div>
    }
}

#[component]
fn SpotifyEmbedInner(track_id: String) -> impl IntoView {
    view! {
        <iframe
            style="border-radius:12px;border:none;max-width: 400px"
            src=format!("https://open.spotify.com/embed/track/{track_id}")
            width="100%"
            height="96"
            allowfullscreen=""
            allow="autoplay; clipboard-write; fullscreen; picture-in-picture"
        ></iframe>
    }
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
