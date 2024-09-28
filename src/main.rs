#[cfg(feature = "ssr")]
use axum::extract::State;

#[cfg(feature = "ssr")]
use axum::response::sse::{Event, KeepAlive, Sse};

#[cfg(feature = "ssr")]
use futures::{stream, Stream};

#[cfg(feature = "ssr")]
use leptos_sse_chat::state::AppState;

#[cfg(feature = "ssr")]
use axum_extra::{headers::Cookie, TypedHeader};

#[cfg(feature = "ssr")]
async fn chat(
    State(AppState { users, .. }): State<AppState>,
    TypedHeader(cookie): TypedHeader<Cookie>,
) -> Sse<impl Stream<Item = Result<Event, axum::Error>>> {
    use leptos_sse_chat::event::{AppEvent, AppEventResponse, ChatEvent};
    use std::time::Duration;
    use tokio_stream::StreamExt as _;

    let chat_recv = if let Some(name) = cookie.get("name") {
        users
            .send(AppEvent::UserJoined {
                user_name: name.to_string(),
            })
            .await
            .ok()
            .and_then(Result::ok)
            .and_then(|ev| {
                if let AppEventResponse::UserListener(listener) = ev {
                    Some(listener)
                } else {
                    None
                }
            })
    } else {
        None
    };

    let allowed = chat_recv.is_some();

    Sse::new(
        stream::repeat_with(move || {
            let Some(chat_recv) = &chat_recv else {
                return Event::default().json_data(ChatEvent::Heartbeat);
            };

            if let Ok(ev) = chat_recv.try_recv() {
                Event::default().json_data(ev)
            } else {
                Event::default().json_data(ChatEvent::Heartbeat)
            }
        })
        .take_while(move |_| allowed)
        .throttle(Duration::from_secs(1)),
    )
    .keep_alive(KeepAlive::new().text("keep-alive"))
}

#[cfg(feature = "ssr")]
#[actix::main]
async fn main() {
    use axum::{routing::get, Router};
    use leptos::prelude::*;
    use leptos_axum::{generate_route_list, LeptosRoutes};
    use leptos_sse_chat::app::*;

    let conf = get_configuration(None).unwrap();
    let addr = conf.leptos_options.site_addr;
    let leptos_options = conf.leptos_options;
    let routes = generate_route_list(App);

    let app_state = AppState::new(leptos_options);

    let app = Router::new()
        .route("/chat", get(chat))
        .leptos_routes_with_context(
            &app_state,
            routes,
            {
                let app_state = app_state.clone();
                move || provide_context(app_state.clone())
            },
            {
                let leptos_options = app_state.leptos_options.clone();
                move || shell(leptos_options.clone())
            },
        )
        .fallback(leptos_axum::file_and_error_handler::<AppState, _>(shell))
        .with_state(app_state);

    log!("listening on http://{}", &addr);
    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
    axum::serve(listener, app.into_make_service())
        .await
        .unwrap();
}

#[cfg(not(feature = "ssr"))]
pub fn main() {}
