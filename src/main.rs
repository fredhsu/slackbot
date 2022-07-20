use slack::*;
use tungstenite::Message; //websocket

mod slack;

#[tokio::main]
async fn main() -> Result<(), reqwest::Error> {
    // Get token from env var?
    let slack_token = slack::Client::get_token_from_file("tokens/slack.token").unwrap();
    //let slack_token = config.slack.token;
    let mut slack = slack::Client::new(slack_token);

    slack.connect().await.unwrap();
    loop {
        let msg = slack.receive_message().await.unwrap();
        match msg {
            Message::Text(t) => handle_text(&t, &mut slack).await,
            Message::Binary(_) => println!("binary"),
            Message::Ping(_p) => {}
            Message::Pong(_p) => {}
            Message::Close(_) => break,
        }
    }
    Ok(())
}


async fn handle_text(message: &str, slack: &mut slack::Client) {
    let socket_event = slack::parse_message(message);
    match socket_event {
        slack::SocketEvent::EventsApi {
            payload,
            envelope_id: _,
            accepts_response_payload: _,
        } => {
            println!("{:?}", payload);
        }
        slack::SocketEvent::SlashCommands {
            payload,
            envelope_id,
            accepts_response_payload: _,
        } => {
            handle_slash_command(slack, payload, envelope_id).await;
        }
        slack::SocketEvent::Interactive {
            payload,
            envelope_id: _,
            accepts_response_payload: _,
        } => {
            println!("Received interactive: {:?}", payload);
            handle_interactive(payload).await;
            println!("response sent");
        }
    }
}

async fn handle_interactive(payload: slack::Interactive) {
    println!("Received interactive with actions {:?}", payload.actions);
    let text = format!(
        "Updated with segement ID {}",
        payload.actions.first().unwrap().selected_option.text.text
    );
    let message = slack::MessagePayload {
        text,
        blocks: None,
        thread_ts: None,
        mrkdwn: false,
    };
    // Resposne to an interactive action is via response_url which is specific to the action and will tie into the block that sent the action
    let response_json = serde_json::to_string(&message).unwrap();
    println!("responding to : {}", &payload.response_url);
    let client = reqwest::Client::new();
    client
        .post(&payload.response_url)
        .body(response_json)
        .send()
        .await
        .unwrap();
    //TODO remove semicolon and make this return value
}

// Matches possible slash commands
// TODO: use an enum for commands
async fn handle_slash_command(
    slack: &mut slack::Client,
    payload: slack::SlashCommand,
    envelope_id: String,
) {
    let command = &payload.get_command();
    match command.as_str() {
        "addservice" => todo!(),
        "addsubnet" => todo!(),
        _ => println!("Unknown command {}", command),
    }
}

async fn add_service(payload: &str, envelope_id: &str, slack: &mut slack::Client) {
    // Parse payload to figure out what service, then what additional information
    let tokens = payload.split_whitespace();
    println!("add_service: tokens: {:?}", &tokens);
    // match for the first token of payload

    let resp_text = format!(
            "Adding service {}, requesting approval",
           "dns" 
        );
    let block2 = Block::new_section(TextBlock::new_mrkdwn(resp_text));
    let blocks = vec![block2];
    let payload = BlockPayload::new(blocks);
    slack.send_response(envelope_id, payload);
}

