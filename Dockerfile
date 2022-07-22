FROM rust:1.62

WORKDIR /usr/src/slackbot
COPY . .

RUN cargo install --path .

CMD ["slackbot"]
