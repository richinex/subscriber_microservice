use actix::Addr;
use actix_web::{web, App, HttpResponse, HttpServer, Responder};
use std::sync::{Arc, Mutex};
use tokio::time::{sleep, Duration};
use serde::{Deserialize, Serialize};
use log:: error;
use actix::Actor;

use reqwest::Error as ReqwestError;

mod appstate;
mod websocket;
use appstate::AppState;
use websocket::{config_ws, ConfigUpdateMsg, WsManager};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
struct Config {
    feature_flag: bool,
    logging_level: String,
    maintenance_mode: bool,
    api_rate_limit: u32,
}

async fn fetch_and_update_config(app_state: web::Data<AppState>, ws_manager: Addr<WsManager>) -> Result<Config, ReqwestError> {
    let url = "http://localhost:8080/config";
    let client = reqwest::Client::new();
    let resp = client.get(url).send().await?;
    let config: Config = resp.json().await?;

    // Update the shared state
    {
        let mut config_lock = app_state.config.lock().unwrap();
        *config_lock = Some(config.clone()); // Clone config for internal state update
    }

    // Send the cloned config to the WsManager for broadcasting
    ws_manager.do_send(ConfigUpdateMsg { config: config.clone() }); // Clone config for messaging

    Ok(config)
}


async fn display_balls(data: web::Data<AppState>) -> impl Responder {
    let config_lock = data.config.lock().unwrap();
    let ball_color = match &*config_lock {
        Some(config) if config.feature_flag => "green",
        _ => "blue",
    };

    // Generate multiple balls with random positions
    let balls_html = (0..50) // Generate 50 balls for example
        .map(|_| {
            let left = rand::random::<u8>() % 100; // Random position from 0% to 100% of the screen width
            let top = rand::random::<u8>() % 100; // Random position from 0% to 100% of the screen height
            format!(
                "<div style='position: absolute; width: 30px; height: 30px; border-radius: 15px; background-color: {}; left: {}%; top: {}%;'></div>",
                ball_color, left, top
            )
        })
        .collect::<Vec<_>>()
        .join("\n");

    // HTML content including the WebSocket script
    let html = format!(
        "<!DOCTYPE html>
        <html>
        <head>
            <title>Balls Color</title>
            <style>
                body {{ margin: 0; overflow: hidden; }}
                div {{ position: absolute; border-radius: 15px; width: 30px; height: 30px; }}
            </style>
        </head>
        <body>
            {}
            <script>
                var conn = new WebSocket('ws://localhost:8081/ws/');
                conn.onopen = function() {{
                    console.log('WebSocket connection established');
                }};
                conn.onerror = function(error) {{
                    console.error('WebSocket Error:', error);
                }};
                conn.onmessage = function(evt) {{
                    var config = JSON.parse(evt.data);
                    console.log('Received config:', config);
                    document.querySelectorAll('div').forEach(div => {{
                        div.style.backgroundColor = config.feature_flag ? 'green' : 'blue';
                    }});
                }};
                conn.onclose = function() {{
                    console.log('WebSocket connection closed');
                }};

                // Function to randomly change the position of each ball
                function moveBalls() {{
                    document.querySelectorAll('div').forEach(function(div) {{
                        var newX = Math.floor(Math.random() * window.innerWidth);
                        var newY = Math.floor(Math.random() * window.innerHeight);
                        div.style.left = newX + 'px';
                        div.style.top = newY + 'px';
                    }});
                }}

                // Call moveBalls every 1000 milliseconds to animate the balls
                setInterval(moveBalls, 1000);
            </script>
        </body>
        </html>",
        balls_html
    );


    HttpResponse::Ok().content_type("text/html").body(html)
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    std::env::set_var("RUST_LOG", "actix_web=info");
    env_logger::init();

    let app_state = web::Data::new(AppState {
        config: Arc::new(Mutex::new(None)),
    });

    // Correctly start the WsManager actor and get its address
    let ws_manager_addr = WsManager::new().start();

    let app_state_cloned = app_state.clone();
    let ws_manager_cloned = ws_manager_addr.clone();
    tokio::spawn(async move {
        loop {
            // Assuming fetch_and_update_config is defined and correctly accepts an Addr<WsManager>
            if let Err(e) = fetch_and_update_config(app_state_cloned.clone(), ws_manager_cloned.clone()).await {
                error!("Failed to fetch config: {}", e);
            }
            sleep(Duration::from_secs(5)).await;
        }
    });

    HttpServer::new(move || {
        App::new()
            .app_data(app_state.clone())
            // Ensure you use `.app_data` for the ws_manager_addr if using Actix Web 3.x or newer
            .app_data(web::Data::new(ws_manager_addr.clone())) // Correctly pass the WsManager address to the app
            .route("/ws/", web::get().to(config_ws))
            .route("/", web::get().to(display_balls))
    })
    .bind("127.0.0.1:8081")?
    .run()
    .await
}
