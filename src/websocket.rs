
use actix::{Actor, Addr, AsyncContext, Context, Handler, Message, Running, StreamHandler};
use actix_web_actors::ws::{self, WebsocketContext};
use log::{debug, info, error};
use serde::{Serialize, Deserialize};
use std::sync::{Arc, Mutex};
use actix_web::{web, HttpRequest, HttpResponse};
use std::collections::HashSet;

use crate::{appstate::AppState, Config};

trait WsMessage {
    fn as_text(&self) -> String;
}

#[derive(Debug, Serialize, Deserialize, Clone, Message)]
#[rtype(result = "()")]
pub struct GenericWsMessage {
    pub config: Config,
}

impl WsMessage for GenericWsMessage {
    fn as_text(&self) -> String {
        serde_json::to_string(self).unwrap()
    }
}

pub struct WsManager {
    sessions: HashSet<Addr<ConfigWs>>,
}

impl WsManager {
    pub fn new() -> Self {
        Self { sessions: HashSet::new() }
    }
}

impl Default for WsManager {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Message)]
#[rtype(result = "()")]
pub struct Connect {
    pub addr: Addr<ConfigWs>,
}

#[derive(Message)]
#[rtype(result = "()")]
pub struct Disconnect {
    pub addr: Addr<ConfigWs>,
}

impl Actor for WsManager {
    type Context = Context<Self>;
}

impl Handler<Connect> for WsManager {
    type Result = ();

    fn handle(&mut self, msg: Connect, _: &mut Self::Context) {
        info!("New client connected: {:?}", msg.addr);
        self.sessions.insert(msg.addr);
    }
}

impl Handler<Disconnect> for WsManager {
    type Result = ();

    fn handle(&mut self, msg: Disconnect, _: &mut Self::Context) {
        info!("Client disconnected: {:?}", msg.addr);
        self.sessions.remove(&msg.addr);
    }
}

impl Handler<GenericWsMessage> for WsManager {
    type Result = ();

    fn handle(&mut self, msg: GenericWsMessage, _: &mut Self::Context) {
        debug!("Broadcasting message: {:?}", msg);
        for addr in self.sessions.iter() {
            addr.do_send(msg.clone());
        }
    }
}

pub struct ConfigWs {
    config: Arc<Mutex<Option<Config>>>,
    ws_manager: Addr<WsManager>,
}

impl Actor for ConfigWs {
    type Context = WebsocketContext<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        let addr = ctx.address();
        self.ws_manager.do_send(Connect { addr });

        // Send the current configuration to the client.
        self.send_current_config(ctx); // Assuming send_current_config is implemented.
    }

    fn stopping(&mut self, ctx: &mut Self::Context) -> Running {
        let addr = ctx.address();
        self.ws_manager.do_send(Disconnect { addr });
        Running::Stop
    }
}


// Utility methods for ConfigWs actor.
impl ConfigWs {
    // This method now sends the current ball configuration to the client.
    fn send_current_config(&self, ctx: &mut WebsocketContext<Self>) {
        let config_lock = self.config.lock().unwrap(); // Lock and access shared config state.
        if let Some(config) = &*config_lock {
            // Serialize the current config to a JSON string
            let config_json = serde_json::to_string(config).expect("Failed to serialize config");
            // Send the serialized config to the client
            ctx.text(config_json);
        } else {
            // Optionally, handle the case where config is not set
            ctx.text("{\"error\": \"Configuration not available.\"}");
        }
    }
}


// Handle incoming WebSocket messages, responding or acting based on the message type.
impl StreamHandler<Result<ws::Message, ws::ProtocolError>> for ConfigWs {
    fn handle(&mut self, msg: Result<ws::Message, ws::ProtocolError>, ctx: &mut Self::Context) {
        match msg {
            Ok(ws::Message::Text(text)) => {
                let message_string = text.to_string(); // Convert ByteString to String

                // Compare the string directly instead of using String::from in the match arm
                if message_string == "get_config" {
                    // Send the current configuration to the client
                    self.send_current_config(ctx);
                } else {
                    // Log unexpected text messages or handle them as needed
                    log::warn!("Received unexpected text message: {}", message_string);
                }
            },

            Err(e) => {
                // Log or handle protocol errors
                log::error!("WebSocket protocol error: {:?}", e);
            },
            // You don't need an exhaustive match here since you've covered all variants of ws::Message
            _ => (), // Ignore other message types (Close, Ping, Pong) or handle as needed
        }
    }
}



// Handle incoming configuration update messages, updating the shared config state and notifying clients.
impl Handler<GenericWsMessage> for ConfigWs {
    type Result = ();

    fn handle(&mut self, msg: GenericWsMessage, ctx: &mut Self::Context) {
        // Update the local configuration based on the message
        let mut config_lock = self.config.lock().unwrap();
        *config_lock = Some(msg.config.clone());

        // Optionally, respond back to the client to confirm the update
        let confirmation = serde_json::to_string(&msg.config).expect("Failed to serialize config");
        ctx.text(confirmation);
    }
}


pub async fn config_ws(req: HttpRequest, stream: web::Payload, data: web::Data<AppState>, ws_manager: web::Data<Addr<WsManager>>) -> HttpResponse {
    debug!("Starting WebSocket session for request: {:?}", req);
    let actor = ConfigWs {
        config: data.config.clone(),
        ws_manager: ws_manager.get_ref().clone(),
    };
    ws::start(actor, &req, stream)
        .unwrap_or_else(|e| {
            error!("Error starting WebSocket session: {:?}", e);
            HttpResponse::InternalServerError().finish()
        })
}