use embassy_net::tcp::TcpSocket;
use esp_hal::gpio::Output;
use esp_println::{self as _, println};
use esp_radio::esp_now::BROADCAST_ADDRESS;
use esp_radio::esp_now::EspNow;
#[derive(Debug)]
pub enum NetworkingError {
    InvalidPacketFormat,
    UnsupportedHTTPMethod,
    ReadError(embassy_net::tcp::Error),
    InvalidUTF8,
    UpgradeRequestFailed(embassy_net::tcp::Error),
    BufferOverflow,
}
use base64::Engine;
use embedded_io_async::Write;

#[derive(Debug, PartialEq, Eq)]
pub enum HttpMethod {
    Get,
    Post,
}
#[derive(Debug, PartialEq, Eq)]
pub enum RequestType {
    Standard(HttpMethod),
    Upgrade(heapless::String<24>),
}

pub async fn read_socket(socket: &mut TcpSocket<'_>) -> Result<RequestType, NetworkingError> {
    let mut buffer = [0u8; 1024];
    let mut pos = 0;
    loop {
        match socket.read(&mut buffer).await {
            //EOF
            Ok(0) => {
                break;
            }
            Ok(len) => {
                let content = core::str::from_utf8(&buffer[..(pos + len)])
                    .map_err(|_| NetworkingError::InvalidUTF8)?;
                pos += len;
                if content.contains("\r\n\r\n") {
                    break;
                }
            }
            Err(e) => {
                return Err(NetworkingError::ReadError(e));
            }
        };
    }
    let content =
        core::str::from_utf8(&buffer[..(pos)]).map_err(|_| NetworkingError::InvalidUTF8)?;
    println!("{}", content);
    let method = RequestType::try_from(content)?;
    Ok(method)
}

impl TryFrom<&str> for RequestType {
    type Error = NetworkingError;
    fn try_from(content: &str) -> Result<Self, Self::Error> {
        if content.starts_with("GET ") {
            if content.contains("Upgrade: websocket") {
                //Sec-WebSocket-Key: t9FAqEj7xvCOE8wKc2ZpbQ==
                let pat = "Sec-WebSocket-Key: ";
                let key_start = content
                    .find(pat)
                    .ok_or(NetworkingError::InvalidPacketFormat)?
                    + pat.len();
                let remainder = &content[key_start..];
                let key_end = remainder
                    .find("\r\n")
                    .ok_or(NetworkingError::InvalidPacketFormat)?;
                let key_slice = &remainder[..key_end];
                if key_slice.len() != 24 {
                    return Err(NetworkingError::InvalidPacketFormat);
                }
                let key: heapless::String<24> = heapless::String::try_from(key_slice)
                    .map_err(|_| NetworkingError::InvalidPacketFormat)?;
                return Ok(RequestType::Upgrade(key));
            }
            Ok(RequestType::Standard(HttpMethod::Get))
        } else if content.starts_with("POST ") {
            Ok(RequestType::Standard(HttpMethod::Post))
        } else {
            Err(NetworkingError::UnsupportedHTTPMethod)
        }
    }
}
use mcu_comms::Deserialize;
use mcu_comms::Serialize;
use mcu_comms::aesccm::Encrypt;
use sha1::Digest;
pub async fn approve_ws<const N: usize>(
    socket: &mut TcpSocket<'_>,
    key: heapless::String<N>,
) -> Result<(), NetworkingError> {
    //Sha1 hashing
    let mut hasher = sha1::Sha1::new();
    hasher.update(key.as_bytes());
    hasher.update("258EAFA5-E914-47DA-95CA-C5AB0DC85B11");
    let hashed = hasher.finalize();
    // Base 64
    let mut b64_buf = [0_u8; 64];
    let encoded_len = base64::engine::general_purpose::STANDARD
        .encode_slice(hashed, &mut b64_buf)
        .map_err(|_| NetworkingError::InvalidUTF8)?;

    let converted_key =
        core::str::from_utf8(&b64_buf[..encoded_len]).map_err(|_| NetworkingError::InvalidUTF8)?;

    let mut response = heapless::String::<256>::new();
    if response
        .push_str("HTTP/1.1 101 Switching Protocols\r\n")
        .is_err()
        || response.push_str("Upgrade: websocket\r\n").is_err()
        || response.push_str("Connection: Upgrade\r\n").is_err()
        || response.push_str("Sec-WebSocket-Accept: ").is_err()
        || response.push_str(converted_key).is_err()
        || response.push_str("\r\n\r\n").is_err()
    {
        return Err(NetworkingError::BufferOverflow);
    }

    socket
        .write_all(response.as_bytes())
        .await
        .map_err(|e| NetworkingError::UpgradeRequestFailed(e))?;

    socket
        .flush()
        .await
        .map_err(|e| NetworkingError::UpgradeRequestFailed(e))?;
    Ok(())
}

#[derive(Debug, Serialize, Deserialize)]
enum Command {
    On(Component),
    Off(Component),
}
#[derive(Debug, Serialize, Deserialize)]
enum Component {
    Led(u8),
}

use mcu_comms::aesccm::{AESCCM, MacAddr, PacketData, PacketView};
pub async fn serve_ws<T: Encrypt>(
    socket: &mut TcpSocket<'_>,
    led: &mut Output<'static>,
    _espnow: &mut EspNow<'static>,
    aesccm: &mut AESCCM<T>,
) {
    let mut buf = [0u8; 1024];

    loop {
        match socket.read(&mut buf).await {
            Ok(0) => {
                break;
            }
            Ok(len) => {
                let content = match decode_payload(&mut buf, len) {
                    Err(e) => {
                        println!("{:?}", e);
                        break;
                    }
                    Ok(slice) => slice,
                };
                println!("content: {}", content);
                led.toggle();
                let packet_data = PacketData::new(
                    MacAddr::default(),
                    0b00000000,
                    Command::On(Component::Led(0)),
                );
                println!("Before: {:?}", packet_data);
                let mut encrypted = match aesccm.encrypt(&packet_data) {
                    Err(e) => {
                        println!("{:?}", e);
                        continue;
                    }
                    Ok(data) => data,
                };
                let decrypted = match aesccm.decrypt::<Command>(&mut encrypted.bytes_mut()) {
                    Err(e) => {
                        println!("decryption error {:?}", e);
                        continue;
                    }
                    Ok(data) => data,
                };
                println!("Decrypted: {:?}", decrypted);
            }
            Err(e) => {
                println!("{}", e);
                break;
            }
        };
    }
}
//buf[0] fin bit
//buf[1] bit len
//buf[2..5] xor key
//buf[6..] payload
const S_PAYLOAD_START: usize = 6;
const M_PAYLOAD_START: usize = 8;
const L_PAYLOAD_START: usize = 14;
fn decode_payload(buf: &mut [u8; 1024], len: usize) -> Result<&str, NetworkingError> {
    if len <= S_PAYLOAD_START {
        let op_code = buf[0] & 0x0F;
        if op_code == 0b1000 {
            return Err(NetworkingError::ReadError(
                embassy_net::tcp::Error::ConnectionReset,
            ));
        }
        return Err(NetworkingError::InvalidPacketFormat);
    }
    let payload_start = match buf[1] & 0x7F {
        0..=125 => S_PAYLOAD_START,
        126 => M_PAYLOAD_START,
        127 => L_PAYLOAD_START,
        _ => return Err(NetworkingError::InvalidPacketFormat),
    };
    let is_xored = (buf[1] & 0x80) != 0;
    if is_xored {
        let mask_start = payload_start - 4;
        let keys_mask = [
            buf[mask_start],
            buf[mask_start + 1],
            buf[mask_start + 2],
            buf[mask_start + 3],
        ];
        let payload = &mut buf[payload_start..len];
        for i in 0..payload.len() {
            let key = keys_mask[i % 4];
            payload[i] ^= key;
        }
    }
    let payload = &mut buf[payload_start..len];
    let content = core::str::from_utf8(payload).map_err(|_| NetworkingError::InvalidUTF8)?;
    Ok(content)
}
