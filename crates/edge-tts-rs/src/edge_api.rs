use std::error::Error;

use std::str::FromStr;
use websocket::client::ParseError;
use websocket::header::{Headers, UserAgent};
use websocket::OwnedMessage::Text;
use websocket::{Message, OwnedMessage};

use crate::util::{gen_muid, gen_request_id, gen_sec_ms_gec, now_millis};
use log::trace;
use websocket::stream::sync::NetworkStream;
use websocket::sync::Client;
use websocket::url::Url;

type TTSSocket = Client<Box<dyn NetworkStream + Send>>;

const TRUSTED_CLIENT_TOKEN: &str = "6A5AA1D4EAFF4E9FB37E23D68491D6F4";
const SEC_MS_GEC_VERSION: &str = "1-143.0.3650.75";
const EDGE_EXTENSION_ORIGIN: &str = "chrome-extension://jdiccldimpdaibmpdkjnbmckianbfold";
const EDGE_USER_AGENT: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/143.0.0.0 Safari/537.36 Edg/143.0.0.0";

pub trait TTS {
    /// connect TTS service
    fn connect(&self) -> Result<TTSSocket, Box<dyn Error>>;
}

pub struct EdgeTTS {
    pub(crate) config: EdgeTTSConfig,
    request_id: String,
    headers: Headers,
}

const PATH_AUDIO: &[u8] = "Path:audio\r\n".as_bytes();
const HEADER_END: &[u8] = "\r\n\r\n".as_bytes();

pub struct EdgeTTSConfig {
    /// # refer
    /// https://speech.platform.bing.com/consumer/speech/synthesize/readaloud/voices/list?trustedclienttoken=6A5AA1D4EAFF4E9FB37E23D68491D6F4
    // todo check the output format
    pub output_format: String,
    pub voice_name: String,
    /// Indicates the speaking rate of the text. Speaking rate can be applied at the word or sentence level. The rate changes should be within 0.5 to 2 times the original audio.
    /// # refer
    /// https://learn.microsoft.com/en-su/azure/ai-services/speech-service/speech-synthesis-markup-voice#adjust-prosody
    /// # scale
    /// -1 is -1%
    pub rate: i16,
    /// Indicates the baseline pitch for the text. Pitch changes can be applied at the sentence level. The pitch changes should be within 0.5 to 1.5 times the original audio.
    /// # scale
    /// -1 is -1%
    pub pitch: i16,
    /// search "Path:audio\r\n" bytes in input, return index
    binary_context_slice_match: fn(&[u8]) -> usize,
}
impl Default for EdgeTTSConfig {
    fn default() -> Self {
        Self {
            output_format: String::from("audio-24khz-96kbitrate-mono-mp3"),
            voice_name: String::from("zh-CN-XiaoxiaoNeural"),
            rate: 0,
            pitch: 0,
            binary_context_slice_match: audio_payload_start,
        }
    }
}

impl EdgeTTSConfig {
    fn to_config_message(&self) -> String {
        let json_first = r#"{"context": {"synthesis": {"audio": {"metadataoptions": {"sentenceBoundaryEnabled": "false","wordBoundaryEnabled": "false"},"outputFormat": "#;
        let json_last = r#"}}}}"#;
        let json = format!(r#"{}"{}"{}"#, json_first, self.output_format, json_last);
        let string = format!(
            "X-Timestamp:{}\r\nContent-Type:application/json; charset=utf-8\r\nPath:speech.config\r\n\r\n{}",
            now_millis(), json
        );
        trace!("speech config {}", string);
        string
    }
    pub(crate) fn to_ssml(&self, content: String) -> String {
        format!(
            r#"<speak xmlns="http://www.w3.org/2001/10/synthesis" xmlns:mstts="http://www.w3.org/2001/mstts" xmlns:emo="http://www.w3.org/2009/10/emotionml" version="1.0" xml:lang="en-US"><voice name="{}"><prosody rate="{}%" pitch="{}%">{}</prosody ></voice > </speak >"#,
            self.voice_name, self.rate, self.pitch, content
        )
    }
}

impl EdgeTTS {
    pub fn new(config: EdgeTTSConfig) -> Self {
        let mut headers = Headers::new();
        headers.set(UserAgent(EDGE_USER_AGENT.to_owned()));
        headers.set_raw("Pragma", vec![b"no-cache".to_vec()]);
        headers.set_raw("Cache-Control", vec![b"no-cache".to_vec()]);
        headers.set_raw("Accept-Encoding", vec![b"gzip, deflate, br, zstd".to_vec()]);
        headers.set_raw("Accept-Language", vec![b"en-US,en;q=0.9".to_vec()]);
        headers.set_raw("Sec-WebSocket-Version", vec![b"13".to_vec()]);
        headers.set_raw("Cookie", vec![format!("muid={};", gen_muid()).into_bytes()]);
        Self {
            config,
            request_id: gen_request_id(),
            headers,
        }
    }
    pub(crate) fn connect_url(&self) -> Result<Url, ParseError> {
        Url::from_str(format!(
            "wss://speech.platform.bing.com/consumer/speech/synthesize/readaloud/edge/v1?TrustedClientToken={}&ConnectionId={}&Sec-MS-GEC={}&Sec-MS-GEC-Version={}",
            TRUSTED_CLIENT_TOKEN,
            self.request_id,
            gen_sec_ms_gec(),
            SEC_MS_GEC_VERSION
        ).as_str())
    }

    /// send text to TTS simple method wrapper
    ///
    /// # Arguments
    ///
    /// * `client`: connected EdgeTTS WebSocket
    /// * `content`: send text
    ///
    /// returns: Result<Vec<u8, Global>, Box<dyn Error, Global>>
    ///
    /// # Examples
    ///
    /// ```
    /// use std::error::Error;
    /// use std::future::Future;
    /// use edge_tts_rs::edge_api::{EdgeTTS, EdgeTTSConfig, TTS};
    ///
    ///
    /// let tts = EdgeTTS::new(EdgeTTSConfig::default());
    /// let mut client = tts.connect().unwrap();
    /// // async {  tts.send(&mut client,"Hello").await};
    /// ```
    pub async fn send_content(
        &self,
        client: &mut TTSSocket,
        content: String,
    ) -> Result<Vec<u8>, Box<dyn Error>> {
        let ssml = self.config.to_ssml(content);
        self.send_ssml(client, ssml).await
    }

    /// send ssml to TTS simple method wrapper
    ///
    /// # Arguments
    ///
    /// * `client`: connected EdgeTTS WebSocket
    /// * `content`: send text
    ///
    /// returns: Result<Vec<u8, Global>, Box<dyn Error, Global>>
    ///
    /// # Examples
    ///
    /// ```
    /// use std::error::Error;
    /// use std::future::Future;
    /// use edge_tts_rs::edge_api::{EdgeTTS, EdgeTTSConfig, TTS};
    ///
    /// let ssml = String::from("");
    /// let tts = EdgeTTS::new(EdgeTTSConfig::default());
    /// let mut client = tts.connect().unwrap();
    /// // async {  tts.send(&mut client,ssml).await};
    /// ```
    pub async fn send_ssml(
        &self,
        client: &mut TTSSocket,
        ssml: String,
    ) -> Result<Vec<u8>, Box<dyn Error>> {
        trace!("ssml prepared: {}", ssml);

        let message = Message::text(format!("X-Timestamp:{}\r\nX-RequestId:{}\r\nContent-Type:application/ssml+xml\r\nPath:ssml\r\n\r\n{}",now_millis(),gen_request_id(),ssml));
        client.send_message(&message)?;

        let mut voice_binary: Vec<u8> = Vec::with_capacity(1024);
        let mut flag: bool = false;
        'l: loop {
            let resp = client.recv_message()?;
            match resp {
                Text(resp) => {
                    trace!("{}", resp);
                    // todo receive example:
                    // keynote: turn.start 1. Text("X-RequestId:ef0e87998a3d4115a0be2e637f5aaed8\r\nContent-Type:application/json; charset=utf-8\r\nPath:turn.start\r\n\r\n{\n  \"context\": {\n    \"serviceTag\": \"57be03b6dae64e2ab82972e81f796ba0\"\n  }\n}")
                    // 2. Text("X-RequestId:ef0e87998a3d4115a0be2e637f5aaed8\r\nContent-Type:application/json; charset=utf-8\r\nPath:response\r\n\r\n{\"context\":{\"serviceTag\":\"57be03b6dae64e2ab82972e81f796ba0\"},\"audio\":{\"type\":\"inline\",\"streamId\":\"0B38A07F8AE1437AB16D46C71CF3ECBB\"}}")
                    // voice info:  3. Text("X-RequestId:ef0e87998a3d4115a0be2e637f5aaed8\r\nContent-Type:application/json; charset=utf-8\r\nPath:audio.metadata\r\n\r\n{\n  \"Metadata\": [\n    {\n      \"Type\": \"WordBoundary\",\n      \"Data\": {\n        \"Offset\": 1000000,\n        \"Duration\": 5625000,\n        \"text\": {\n          \"Text\": \"Hello\",\n          \"Length\": 5,\n          \"BoundaryType\": \"WordBoundary\"\n        }\n      }\n    }\n  ]\n}")
                    // keynote: turn.end 4. message response: Text("X-RequestId:ef0e87998a3d4115a0be2e637f5aaed8\r\nContent-Type:application/json; charset=utf-8\r\nPath:turn.end\r\n\r\n{}")
                    if resp.contains("turn.start") {
                        flag = true;
                    } else if resp.contains("turn.end") {
                        break 'l;
                    }
                }
                OwnedMessage::Binary(resp) => {
                    trace!("{:?}", resp);
                    if flag {
                        let x = (self.config.binary_context_slice_match)(&resp);
                        let mut resp = resp[x..].to_vec();
                        voice_binary.append(&mut resp);
                    }
                }
                OwnedMessage::Close(resp) => {
                    trace!("{:?}", resp);
                    return match resp {
                        None => Err("the socket closed".to_string().into()),
                        Some(reason) => Err(reason.reason.into()),
                    };
                }
                _ => break 'l,
            }
        }
        Ok(voice_binary)
    }
}
impl TTS for EdgeTTS {
    fn connect(&self) -> Result<TTSSocket, Box<dyn Error>> {
        let url = self.connect_url()?;
        let mut builder = websocket::ClientBuilder::from_url(&url)
            .custom_headers(&self.headers)
            .origin(EDGE_EXTENSION_ORIGIN.to_string());

        match builder.connect(None) {
            Ok(mut c) => {
                let message = Message::text(self.config.to_config_message());
                c.send_message(&message)?;
                Ok(c)
            }
            Err(e) => Err(Box::new(e)),
        }
    }
}

fn audio_payload_start(message: &[u8]) -> usize {
    if message.len() >= 2 {
        let header_len = u16::from_be_bytes([message[0], message[1]]) as usize;
        let header_start = 2;
        let header_end = header_start + header_len;

        if header_end <= message.len() {
            let header = &message[header_start..header_end];
            if header
                .windows(PATH_AUDIO.len())
                .any(|window| window == PATH_AUDIO)
            {
                return header_end;
            }
        }
    }

    message
        .windows(HEADER_END.len())
        .position(|window| window == HEADER_END)
        .map(|index| index + HEADER_END.len())
        .or_else(|| {
            message
                .windows(PATH_AUDIO.len())
                .position(|window| window == PATH_AUDIO)
                .map(|index| index + PATH_AUDIO.len())
        })
        .unwrap_or(0)
}
