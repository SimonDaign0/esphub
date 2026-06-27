use serde::{Deserialize, Serialize};
#[derive(Serialize, Deserialize, Debug)]
pub enum PacketError {
    Authentication,
    InvalidFormat,
    BufferOverflow,
    AESCounterOverflow,
    Duplicate,
    Corrupted,
}
#[derive(Serialize, Deserialize, Debug)]
pub enum Command {
    Toggle(Component),
    On(Component),
    Off(Component),
}
#[derive(Serialize, Deserialize, Debug)]
pub enum Component {
    Led(u8),
}
