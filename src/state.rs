use actix::{Actor, Addr};
use axum::extract::FromRef;
use leptos::config::LeptosOptions;
use serde::Serialize;

use crate::event::Users;

#[derive(Serialize, Clone)]
pub struct Message {
    pub sender_name: String,
    pub message: String,
}

#[derive(Serialize, Clone)]
pub struct User {
    pub name: String,
}

#[derive(FromRef, Clone)]
pub struct AppState {
    pub leptos_options: LeptosOptions,
    pub users: Addr<Users>,
}

impl AppState {
    pub fn new(leptos_options: LeptosOptions) -> Self {
        Self {
            leptos_options,
            users: Users::default().start(),
        }
    }
}
