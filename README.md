# Slackbot for EMEA demo

## Helpers

* run-nats-docker.sh : run the nats-server docker container on localhost for testing

## TODO

[] Use different docker for build and run
[] Use alpine Linux base image
[] Use threads to subscribe
[] Create deployment yaml for k8s cluster

### Notes on building docker image

rust: `<version>-alpine`

```rust
FROM rust:1.40 as builder
WORKDIR /usr/src/myapp
COPY . .
RUN cargo install --path .

FROM debian:buster-slim
RUN apt-get update && apt-get install -y extra-runtime-dependencies && rm -rf /var/lib/apt/lists/*
COPY --from=builder /usr/local/cargo/bin/myapp /usr/local/bin/myapp
CMD ["myapp"]
```
