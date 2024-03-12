
# Subscriber Microservice for Dynamic Ball Display

This microservice is part of an application that manages the dynamic display of balls with properties such as color, size, and speed being configurable in real-time. It uses Actix Web for serving web requests and Tokio for asynchronous programming.

## Features

- **WebSocket Communication**: Utilizes WebSockets for real-time communication between the client and server, allowing for live updates to the balls' display based on configuration changes.
- **Dynamic Configuration Fetching**: Fetches configuration updates from a central configuration management server [live_config_updates](https://github.com/richinex/live_config_updates) and applies these updates in real-time to the display.
- **Robust Error Handling**: Includes error handling for network requests and configuration parsing to ensure the application remains stable and responsive.

## Getting Started

### Prerequisites

- Rust and Cargo installed on your machine.
- Basic understanding of Rust, asynchronous programming with Tokio, and web development with Actix Web.

### Running the Microservice

1. Clone the [repository](https://github.com/richinex/subscriber_microservice.git) to your local machine

2. Navigate to the subscriber_microservice directory:
   ```bash
   cd subscriber_microservice
   ```
3. Run the microservice using Cargo:
   ```bash
   cargo run
   ```
   This starts the server on `127.0.0.1:8081` and listens for incoming WebSocket connections and configuration updates.

4. Point your browser to the URL and see the balls change based on the configuration applied.

## Configuration

The microservice fetches its initial configuration from a central server (`http://localhost:8080/config`) and listens for real-time updates over WebSocket (`ws://localhost:8081/ws/`). The configuration includes properties like `ball_color`, `ball_size`, `ball_speed`, and `number_of_balls`.

## Endpoints

- **WebSocket `/ws/`**: Accepts WebSocket connections for real-time configuration updates.
- **HTTP GET `/`**: Serves the dynamic ball display page with live updates to ball properties.

## Architecture

This microservice is designed with scalability and real-time performance in mind, leveraging Actix Web's powerful actor system and Tokio's asynchronous runtime. It demonstrates a practical application of modern Rust web development techniques.

## Contributing

Contributions to enhance the functionality, improve error handling, or extend the feature set are welcome. Please feel free to submit pull requests or open issues for discussion.

