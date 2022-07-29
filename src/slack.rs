use reqwest::header::*;
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::{Error, ErrorKind};

use log::info;
use std::net::TcpStream;
use tungstenite::stream::MaybeTlsStream;
use tungstenite::{connect, Message};
use url::Url;

#[derive(Debug)]
pub enum SlackErr {
    Http(reqwest::Error),
    Parse(url::ParseError),
    Connection(std::io::Error),
}

impl From<url::ParseError> for SlackErr {
    fn from(err: url::ParseError) -> Self {
        SlackErr::Parse(err)
    }
}
impl From<reqwest::Error> for SlackErr {
    fn from(err: reqwest::Error) -> Self {
        SlackErr::Http(err)
    }
}
impl From<std::io::Error> for SlackErr {
    fn from(err: std::io::Error) -> Self {
        SlackErr::Connection(err)
    }
}

#[derive(Deserialize, Debug)]
struct AppsConnectionsOpenResponse {
    ok: bool,
    url: String,
    error: Option<String>,
}

#[derive(Deserialize, Debug)]
#[serde(tag = "type")]
pub enum SocketEvent {
    #[serde(rename = "events_api")]
    EventsApi {
        payload: EventCallback,
        envelope_id: String,
        accepts_response_payload: bool,
    },
    #[serde(rename = "slash_commands")]
    SlashCommands {
        payload: SlashCommand,
        envelope_id: String,
        accepts_response_payload: bool,
    },
    #[serde(rename = "interactive")]
    Interactive {
        payload: Interactive,
        envelope_id: String,
        accepts_response_payload: bool,
    },
}
impl SocketEvent {
    pub fn get_envelope_id(self) -> String {
        match self {
            SocketEvent::EventsApi {
                payload,
                envelope_id,
                accepts_response_payload,
            } => envelope_id,
            SocketEvent::SlashCommands {
                payload,
                envelope_id,
                accepts_response_payload,
            } => envelope_id,
            SocketEvent::Interactive {
                payload,
                envelope_id,
                accepts_response_payload,
            } => envelope_id,
        }
    }
}

#[derive(Deserialize, Serialize, Debug)]
pub struct MessagePayload {
    pub text: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub blocks: Option<Vec<Block>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thread_ts: Option<String>,
    pub mrkdwn: bool,
}

#[derive(Deserialize, Debug)]
pub struct Interactive {
    #[serde(rename = "type")]
    interactive_type: String,
    pub actions: Vec<InteractiveAction>,
    pub response_url: String,
}

#[derive(Deserialize, Debug)]
pub struct InteractiveAction {
    #[serde(rename = "type")]
    action_type: String,
    action_id: String,
    block_id: String,
    action_ts: String,
    pub selected_option: SelectedOption,
}

#[derive(Deserialize, Debug)]
pub struct SelectedOption {
    pub text: TextBlock,
    pub value: String,
}
#[derive(Deserialize, Debug)]
pub struct AppMention {
    #[serde(rename = "type")]
    event_type: String,
    user: String,
    text: String,
    ts: String,
    channel: String,
    event_ts: String,
}
#[derive(Deserialize, Debug)]
pub struct EventCallback {
    token: String,
    team_id: String,
    event: AppMention,
    event_id: String,
}
// TODO: make the client specific to WSS either by name of struct or by module
pub struct Client {
    token: String,
    socket: Option<tungstenite::WebSocket<MaybeTlsStream<TcpStream>>>,
}

impl Client {
    pub fn new(token: String) -> Self {
        Client {
            token,
            socket: None,
        }
    }
    async fn get_wss_url(&mut self) -> Result<Url, SlackErr> {
        let base_url = "https://slack.com/api/".to_owned();
        let client = reqwest::Client::new();
        let connection_response = client
            .post(base_url + "apps.connections.open")
            .bearer_auth(&self.token)
            .header(CONTENT_TYPE, "application/x-www-form-urlencoded")
            .send()
            .await?
            .json::<AppsConnectionsOpenResponse>()
            .await?;

        // check if connection was successful
        if !connection_response.ok {
            return Err(SlackErr::Connection(Error::new(
                ErrorKind::Other,
                connection_response.error.unwrap(),
            )));
        }
        let url = Url::parse(&connection_response.url).expect("Could not parse url");
        Ok(url)
    }

    pub async fn connect(&mut self) -> Result<(), Error> {
        let url = self.get_wss_url().await.expect("Could not get wss url");
        let (mut socket, _response) = connect(url).expect("Can't connect");
        let msg = socket.read_message().expect("Error reading message");
        info!("recevied hello: {:?}", msg);
        self.socket = Some(socket);
        Ok(())
    }
    pub fn get_token_from_file(filename: &str) -> Result<String, std::io::Error> {
        let t = fs::read_to_string(filename)?;
        let token = t.trim().to_string();
        Ok(token)
    }
    pub async fn receive_message(&mut self) -> Result<Message, Error> {
        Ok(self
            .socket
            .as_mut()
            .unwrap()
            .read_message()
            .expect("Error reading message"))
    }
    pub fn send_message(&mut self, msg: &str) {
        println!("send message {}", msg);
        self.socket
            .as_mut()
            .unwrap()
            .write_message(Message::Text(msg.into()))
            .unwrap();
    }
    pub fn send_response(&mut self, envelope_id: &str, payload: BlockPayload) {
        let response = Response {
            envelope_id: envelope_id.to_string(),
            payload,
        };
        let response_json = serde_json::to_string(&response).unwrap();
        self.send_message(&response_json);
    }
}

#[derive(Deserialize, Debug)]
pub struct SlashCommand {
    token: String,
    team_id: String,
    team_domain: String,
    channel_id: String,
    channel_name: String,
    user_id: String,
    user_name: String,
    command: String,
    pub text: String,
    api_app_id: String,
    is_enterprise_install: String,
    response_url: String,
    trigger_id: String,
}

impl SlashCommand {
    pub fn get_command(&self) -> String {
        match self.command.strip_prefix('/') {
            Some(s) => s.to_string(),
            None => "".to_string(),
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
struct Response {
    envelope_id: String,
    payload: BlockPayload,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct BlockPayload {
    blocks: Vec<Block>,
}
impl BlockPayload {
    pub fn new(blocks: Vec<Block>) -> Self {
        BlockPayload { blocks }
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Block {
    #[serde(rename = "type")]
    block_type: String,
    text: TextBlock,
    #[serde(skip_serializing_if = "Option::is_none")]
    fields: Option<Vec<TextBlock>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    accessory: Option<StaticSelect>,
}
impl Block {
    pub fn new_section(text: TextBlock) -> Self {
        Block {
            block_type: "section".to_owned(),
            text,
            fields: None,
            accessory: None,
        }
    }
    pub fn add_accessory(&mut self, element: StaticSelect) {
        self.accessory = Some(element);
    }
}
#[derive(Serialize, Deserialize, Debug)]
pub struct Element {}
#[derive(Serialize, Deserialize, Debug)]
pub struct StaticSelect {
    #[serde(rename = "type")]
    element_type: String,
    placeholder: TextBlock,
    action_id: String,
    options: Vec<OptionObject>,
    #[serde(skip_serializing_if = "Option::is_none")]
    option_groups: Option<Vec<OptionObject>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    initial_option: Option<OptionObject>,
    #[serde(skip_serializing_if = "Option::is_none")]
    confirm: Option<ConfirmObject>,
    focus_on_load: bool,
}
impl StaticSelect {
    pub fn new(placeholder: TextBlock, action_id: String, options: Vec<OptionObject>) -> Self {
        StaticSelect {
            element_type: "static_select".to_string(),
            placeholder,
            action_id,
            options,
            option_groups: None,
            initial_option: None,
            confirm: None,
            focus_on_load: false,
        }
    }
}
#[derive(Serialize, Deserialize, Debug)]
pub struct ConfirmObject {}
#[derive(Serialize, Deserialize, Debug)]
pub struct OptionObject {
    text: TextBlock,
    value: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    url: Option<String>,
}

impl OptionObject {
    pub fn new(text: TextBlock, value: String) -> Self {
        OptionObject {
            text,
            value,
            description: None,
            url: None,
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct TextBlock {
    #[serde(rename = "type")]
    text_type: String,
    pub text: String,
}
impl TextBlock {
    pub fn new_plain(text: String) -> TextBlock {
        TextBlock {
            text_type: "plain_text".to_owned(),
            text,
        }
    }
    pub fn new_mrkdwn(text: String) -> TextBlock {
        TextBlock {
            text_type: "mrkdwn".to_owned(),
            text,
        }
    }
}

pub fn handle_slash_command(
    socket: &mut tungstenite::WebSocket<MaybeTlsStream<TcpStream>>,
    envelope_id: String,
) {
    let block1 = Block::new_section(TextBlock::new_mrkdwn("This is a test".to_owned()));
    let block2 = Block::new_section(TextBlock::new_mrkdwn("This is another test".to_owned()));
    let blocks = vec![block1, block2];
    let payload = BlockPayload { blocks };

    // send block back as resposne
    let response = Response {
        envelope_id,
        payload,
    };
    let response_json = serde_json::to_string(&response).unwrap();
    println!("send message {}", &response_json);
    socket.write_message(Message::Text(response_json)).unwrap();
}

pub fn parse_message(s: &str) -> SocketEvent {
    let socket_event: SocketEvent = serde_json::from_str(s).unwrap();
    socket_event
}
