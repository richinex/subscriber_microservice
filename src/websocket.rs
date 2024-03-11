// Import necessary modules and traits from Actix and Serde for WebSocket and async communication, and std for synchronization primitives.
use actix::{Actor, Addr, AsyncContext, Context, Handler, Message, Running, StreamHandler};
use actix_web_actors::ws::{self, WebsocketContext};
use serde::{Serialize, Deserialize};
use std::sync::{Arc, Mutex};
use actix_web::{web, HttpRequest, HttpResponse};
use std::collections::HashSet;

use crate::{appstate::AppState, Config};

// Define a message struct for updating the configuration, serializable for easy WebSocket communication.
#[derive(Serialize, Deserialize, Clone, Message)]
#[rtype(result = "()")]
pub struct ConfigUpdateMsg {
    pub config: Config, // Configuration data to be updated.
}

// Actor for managing WebSocket sessions, tracking all active connections.
pub struct WsManager {
    sessions: HashSet<Addr<ConfigWs>>, // Stores addresses of connected WebSocket clients.
}

impl WsManager {
    // Constructor for WsManager, initializes with no active sessions.
    pub fn new() -> Self {
        Self { sessions: HashSet::new() }
    }
}

// Default trait implementation for WsManager, facilitating easier instantiation.
impl Default for WsManager {
    fn default() -> Self {
        Self::new()
    }
}

// Messages for handling WebSocket connection and disconnection events.
#[derive(Message)]
#[rtype(result = "()")]
pub struct Connect {
    pub addr: Addr<ConfigWs>, // Address of the connecting WebSocket client.
}

#[derive(Message)]
#[rtype(result = "()")]
pub struct Disconnect {
    pub addr: Addr<ConfigWs>, // Address of the disconnecting WebSocket client.
}

// Implement Actor trait for WsManager to integrate with Actix's actor system.
impl Actor for WsManager {
    type Context = Context<Self>;
}

// Handle incoming connect messages, adding new clients to the sessions set.
impl Handler<Connect> for WsManager {
    type Result = ();

    fn handle(&mut self, msg: Connect, _: &mut Self::Context) {
        self.sessions.insert(msg.addr);
    }
}

// Handle incoming disconnect messages, removing clients from the sessions set.
impl Handler<Disconnect> for WsManager {
    type Result = ();

    fn handle(&mut self, msg: Disconnect, _: &mut Self::Context) {
        self.sessions.remove(&msg.addr);
    }
}

// Handle configuration update messages, broadcasting updates to all connected clients.
impl Handler<ConfigUpdateMsg> for WsManager {
    type Result = ();

    fn handle(&mut self, msg: ConfigUpdateMsg, _: &mut Self::Context) {
        for addr in self.sessions.iter() {
            addr.do_send(msg.clone()); // Broadcast update message to each client.
        }
    }
}

// Define the WebSocket handler actor for individual connections.
pub struct ConfigWs {
    config: Arc<Mutex<Option<Config>>>, // Shared configuration state.
    ws_manager: Addr<WsManager>, // Address of the WebSocket manager for session management.
}

// Utility methods for ConfigWs actor.
impl ConfigWs {
    fn send_based_on_config(&self, ctx: &mut WebsocketContext<Self>, message: String) {
        let config_lock = self.config.lock().unwrap(); // Lock and access shared config state.
        if let Some(config) = &*config_lock {
            // Send message based on feature flag in the configuration.
            if config.feature_flag {
                ctx.text(format!("Feature is enabled: {}", message));
            } else {
                ctx.text(format!("Feature is disabled: {}", message));
            }
        }
    }
}

// Implement Actor trait for ConfigWs to work within Actix's actor system.
impl Actor for ConfigWs {
    type Context = WebsocketContext<Self>;

    // Register with WsManager on actor start.
    fn started(&mut self, ctx: &mut Self::Context) {
        let addr = ctx.address();
        self.ws_manager.do_send(Connect { addr });
    }

    // Deregister with WsManager when actor stops.
    fn stopping(&mut self, ctx: &mut Self::Context) -> Running {
        let addr = ctx.address();
        self.ws_manager.do_send(Disconnect { addr });
        Running::Stop
    }
}

// Handle incoming WebSocket messages, responding or acting based on the message type.
impl StreamHandler<Result<ws::Message, ws::ProtocolError>> for ConfigWs {
    fn handle(&mut self, msg: Result<ws::Message, ws::ProtocolError>, ctx: &mut Self::Context) {
        // Echo text messages back to the client or handle other message types as needed.
        if let Ok(ws::Message::Text(text)) = msg {
            let message_string = text.to_string(); // Convert message to string if needed.
            self.send_based_on_config(ctx, message_string);
        }
    }
}

// Handle incoming configuration update messages, updating the shared config state and notifying clients.
impl Handler<ConfigUpdateMsg> for ConfigWs {
    type Result = ();

    fn handle(&mut self, msg: ConfigUpdateMsg, ctx: &mut Self::Context) {
        // Serialize the updated config and send it to the client.
        let config_json = serde_json::to_string(&msg.config).expect("Failed to serialize config");
        ctx.text(config_json);
    }
}

// Asynchronous handler for initiating WebSocket connections, setting up the actor and WebSocket context.
pub async fn config_ws(req: HttpRequest, stream: web::Payload, data: web::Data<AppState>, ws_manager: web::Data<Addr<WsManager>>) -> HttpResponse {
    let actor = ConfigWs {
        config: data.config.clone(), // Clone shared configuration state.
        ws_manager: ws_manager.get_ref().clone(), // Clone manager's address for session management.
    };
    // Start WebSocket actor for the incoming connection.
    ws::start(actor, &req, stream)
        .unwrap_or_else(|_| HttpResponse::InternalServerError().finish())
}
