use std::convert::Infallible;

use actix::prelude::*;
use std::sync::mpsc::{Receiver, Sender};

use crate::app_event::{ChatEvent, ChatMessage};

#[derive(Message)]
#[rtype(result = "Result<AppEventResponse, Infallible>")]
pub enum AppEvent {
    Message { user_name: String, message: String },
    UserJoined { user_name: String },
    UserLeft { user_name: String },
}

impl Message for ChatMessage {
    type Result = Result<(), Infallible>;
}

pub enum AppEventResponse {
    UserListener(Receiver<ChatEvent>),
    Forbidden,
    None,
}

#[derive(Clone, Debug)]
pub struct User {
    name: String,
    event_sender: Sender<ChatEvent>,
}

#[derive(Default, Debug)]
pub struct Users {
    users: Vec<(User, Addr<User>)>,
}

impl Actor for User {
    type Context = Context<Self>;

    fn started(&mut self, _: &mut Self::Context) {}
    fn stopped(&mut self, _: &mut Self::Context) {}
}

impl Actor for Users {
    type Context = Context<Self>;

    fn started(&mut self, _: &mut Self::Context) {}
    fn stopped(&mut self, _: &mut Self::Context) {}
}

impl Handler<ChatMessage> for User {
    type Result = Result<(), Infallible>;

    fn handle(&mut self, msg: ChatMessage, _: &mut Self::Context) -> Self::Result {
        _ = self.event_sender.send(ChatEvent::Message(msg));

        Ok(())
    }
}

impl Handler<AppEvent> for Users {
    type Result = Result<AppEventResponse, Infallible>;

    fn handle(&mut self, msg: AppEvent, _: &mut Self::Context) -> Self::Result {
        match msg {
            AppEvent::UserJoined { user_name } => {
                if self.users.iter().any(|(user, _)| user.name == user_name) {
                    return Ok(AppEventResponse::Forbidden);
                }

                let (event_sender, event_recv) = std::sync::mpsc::channel::<ChatEvent>();

                let user = User {
                    name: user_name,
                    event_sender,
                };

                self.users.push((user.clone(), user.start()));

                return Ok(AppEventResponse::UserListener(event_recv));
            }
            AppEvent::UserLeft { user_name } => {
                if let Some(position) = self
                    .users
                    .iter()
                    .position(|(user, _)| *user.name == user_name)
                {
                    self.users.remove(position);
                }
            }
            AppEvent::Message { user_name, message } => {
                let id = nanoid::nanoid!();

                for (_, actor) in self.users.iter() {
                    actor.do_send(ChatMessage {
                        id: id.clone(),
                        author: user_name.clone(),
                        content: message.clone(),
                    });
                }
            }
        }

        Ok(AppEventResponse::None)
    }
}
