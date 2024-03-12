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
use websocket::{config_ws, GenericWsMessage, WsManager};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
struct Config {
    ball_color: String,    // Color of the balls (e.g., "green", "red", "blue")
    ball_size: u8,         // Diameter of the balls in pixels
    ball_speed: u8,        // Speed of the balls' movement (pixels per animation frame)
    number_of_balls: u8,   // Total number of balls to display
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
    ws_manager.do_send(GenericWsMessage { config: config.clone() }); // Clone config for messaging

    Ok(config)
}


async fn display_balls(data: web::Data<AppState>) -> impl Responder {
    let config_lock = data.config.lock().unwrap();
    let config = match &*config_lock {
        Some(config) => config.clone(),
        None => return HttpResponse::InternalServerError().finish(), // Handle missing config
    };

    // Initial rendering of balls based on the server-side configuration
    let balls_html = (0..config.number_of_balls)
        .map(|_| {
            let left = rand::random::<u8>() % 100;
            let top = rand::random::<u8>() % 100;
            format!(
                "<div class='ball' style='position: absolute; width: {}px; height: {}px; border-radius: {}px; background-color: {}; left: {}%; top: {}%;'></div>",
                config.ball_size, config.ball_size, config.ball_size / 2, config.ball_color, left, top
            )
        })
        .collect::<Vec<_>>()
        .join("\n");

    let html = format!(
        "<!DOCTYPE html>
        <html>
        <head>
            <title>Balls Display</title>
            <style>
                body {{ margin: 0; overflow: hidden; }}
                .ball {{ position: absolute; border-radius: 50%; }}
            </style>
        </head>
        <body>
            {balls_html}
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
                    // Update ball characteristics based on the new config
                    document.querySelectorAll('.ball').forEach(div => {{
                        div.style.backgroundColor = config.ball_color;
                        div.style.width = config.ball_size + 'px';
                        div.style.height = config.ball_size + 'px';
                        div.style.borderRadius = (config.ball_size / 2) + 'px';
                    }});
                    // Adjust the number of balls as needed
                    updateNumberOfBalls(config.number_of_balls, config.ball_size, config.ball_color);
                    // Update the movement speed based on the new configuration
                    currentSpeed = config.ball_speed || defaultSpeed;
                    clearInterval(moveInterval); // Clear the existing interval
                    moveInterval = setInterval(moveBalls, 1000 / currentSpeed); // Set a new interval with updated speed
                }};
                conn.onclose = function() {{
                    console.log('WebSocket connection closed');
                }};

                var defaultSpeed = 5; // Default speed for ball movement
                var currentSpeed = defaultSpeed; // Current speed, initially set to default
                var moveInterval = setInterval(moveBalls, 1000 / currentSpeed); // Initialize ball movement

                function moveBalls() {{
                    document.querySelectorAll('.ball').forEach(function(div) {{
                        var newX = Math.floor(Math.random() * (window.innerWidth - div.offsetWidth));
                        var newY = Math.floor(Math.random() * (window.innerHeight - div.offsetHeight));
                        div.style.left = newX + 'px';
                        div.style.top = newY + 'px';
                    }});
                }}

                function updateNumberOfBalls(newNumberOfBalls, ballSize, ballColor) {{
                    const ballsContainer = document.body;
                    const existingBalls = document.querySelectorAll('.ball');
                    const currentNumberOfBalls = existingBalls.length;

                    // Add balls if new number is greater
                    for (let i = currentNumberOfBalls; i < newNumberOfBalls; i++) {{
                        const div = document.createElement('div');
                        div.className = 'ball';
                        div.style.position = 'absolute';
                        div.style.width = ballSize + 'px';
                        div.style.height = ballSize + 'px';
                        div.style.borderRadius = (ballSize / 2) + 'px';
                        div.style.backgroundColor = ballColor;
                        // Set initial position
                        div.style.left = (Math.random() * window.innerWidth) + 'px';
                        div.style.top = (Math.random() * window.innerHeight) + 'px';
                        ballsContainer.appendChild(div);
                    }}

                    // Remove balls if new number is smaller
                    for (let i = currentNumberOfBalls - 1; i >= newNumberOfBalls; i--) {{
                        existingBalls[i].remove();
                    }}
                }}
            </script>
        </body>
        </html>",
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
