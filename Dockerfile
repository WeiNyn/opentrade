FROM debian:bullseye-slim

# Set the working directory
WORKDIR /usr/local/bin

# Copy the compiled binary from the builder stage
# The binary is located in /usr/src/myapp/target/release/streaming_klines from the previous stage
COPY ./target/release/streaming_klines .
COPY ./target/release/backfill_klines .


# Set the default command to run the application
CMD ["./streaming_klines"]

# You can also expose ports if your application is a web server
# EXPOSE 8000
