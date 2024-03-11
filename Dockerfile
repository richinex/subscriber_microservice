# Use the official Rust image as a parent image
FROM rust:1.58

# Set the working directory in the container
WORKDIR /usr/src/app

# Copy the current directory contents into the container
COPY . .

# Build your program for release
RUN cargo install --path .

# Command to run the executable
CMD ["subscriber_microservice"]
