use crate::enums::{Command, PacketError};
use esp_println::println;
use serde::{Deserialize, Serialize};
pub const TKN: [u8; 3] = [0x2C, 0x0F, 0xDE];
pub const TAG: [u32; 1] = [0x3D9CAF52];
#[derive(Serialize, Deserialize, Debug)]
pub struct EspPayload {
    cmd: Command,
}
impl EspPayload {
    pub fn new(cmd: Command) -> Self {
        Self { cmd }
    }
}

const MAX_5_BYTES: u64 = 0xFF_FF_FF_FF_FF;
pub struct Nonce {
    counter: u64,
}
impl Nonce {
    pub fn inc(&mut self) -> Result<[u8; 5], PacketError> {
        if self.counter >= MAX_5_BYTES {
            return Err(PacketError::AESCounterOverflow);
        }
        self.counter += 1;

        let bytes = self.counter.to_be_bytes();
        let mut result = [0_u8; 5];
        result.copy_from_slice(&bytes[3..8]);

        Ok(result)
    }
    pub fn set(&mut self, new_counter: u64) {
        self.counter = new_counter;
    }
}
use esp_hal::aes::Aes;
// payload field size = size - 1
// Tag size = (size  - 2) / 2
// reserved|ad header?|Tag size| payload field size|
// 0b0|1|011|001
// | 1 ctl byte| 2 payload size | 8 MAC | 5 counter
// Nonce len = 16 - payload size - ctl byte = 13
const CTL_BYTE: u8 = 0b01011001;
const MAX_PAYLOAD_LEN: u16 = 32;
const HEADER_LEN: u16 = 14;
const HEADER_SIZE: usize = 16;
const TAG_SIZE: usize = 8;
const NONCE_SIZE: usize = 13;
const HEADER_DATA: &[u8; HEADER_LEN as usize] = b"Src:Ap Dst:Sta";
pub struct AESCCM {
    rx_nonce: Nonce,
    tx_nonce: Nonce,
    key: [u8; 16],
    aes: Aes<'static>,
}
impl AESCCM {
    pub fn new(aes: Aes<'static>, key: [u8; 16]) -> Self {
        AESCCM {
            rx_nonce: Nonce { counter: 0 },
            tx_nonce: Nonce { counter: 0 },
            key,
            aes,
        }
    }

    pub fn encrypt(&mut self, esp_payload: EspPayload) -> Result<AESCCMPacket, PacketError> {
        let mut payload_buf = [0_u8; MAX_PAYLOAD_LEN as usize];
        let payload_len = postcard::to_slice(&esp_payload, &mut payload_buf)
            .map_err(|_| PacketError::BufferOverflow)?
            .len();
        let padded_tx_len = if payload_len <= 16 {
            16
        } else {
            MAX_PAYLOAD_LEN as usize
        };
        let mac_addr = &[0xFF; 8];
        let nonce = &self.tx_nonce.inc()?;
        //b_block ============================================================
        let mut b_block = [0_u8; 16];
        b_block[0] = CTL_BYTE;
        b_block[1..=8].copy_from_slice(mac_addr);
        b_block[9..=13].copy_from_slice(nonce);
        b_block[14..=15].copy_from_slice(&(padded_tx_len as u16).to_be_bytes());
        let mut head_and_data_payload = [0_u8; HEADER_SIZE + MAX_PAYLOAD_LEN as usize];
        head_and_data_payload[0..=1].copy_from_slice(&HEADER_LEN.to_be_bytes());
        head_and_data_payload[2..HEADER_SIZE].copy_from_slice(HEADER_DATA);
        head_and_data_payload[HEADER_SIZE..HEADER_SIZE + padded_tx_len]
            .copy_from_slice(&payload_buf[..padded_tx_len]);
        //tag gen ============================================================
        self.aes.encrypt(&mut b_block, self.key);
        let (chunks, _) = head_and_data_payload.as_chunks::<16>();
        for chunk in chunks {
            for j in 0..16 {
                b_block[j] ^= chunk[j];
            }
            self.aes.encrypt(&mut b_block, self.key);
        }

        let mut tag = [0_u8; 8];
        tag[0..8].copy_from_slice(&b_block[0..8]);
        //a_block ============================================================
        let mut a_block = [0_u8; 16];
        a_block[0] = 0x02;
        a_block[1..=8].copy_from_slice(mac_addr);
        a_block[9..=13].copy_from_slice(nonce);
        //tag xoring ========================================================
        let mut key_stream = [0_u8; 16];
        key_stream.copy_from_slice(&a_block);
        self.aes.encrypt(&mut key_stream, self.key);
        for i in 0..8 {
            tag[i] ^= key_stream[i];
        }
        //inc counter =================================================
        let (chunks, _) = payload_buf.as_chunks_mut::<16>();
        for chunk in chunks {
            //inc slice as u16
            let mut counter = u16::from_be_bytes([a_block[14], a_block[15]]);
            counter = counter
                .checked_add(1)
                .ok_or(PacketError::AESCounterOverflow)?;
            [a_block[14], a_block[15]] = counter.to_be_bytes();
            //
            key_stream.copy_from_slice(&a_block);
            self.aes.encrypt(&mut key_stream, self.key);
            for j in 0..16 {
                chunk[j] ^= key_stream[j];
            }
        }
        // payload vec ==================================================

        let mut payload_vec = AESCCMPacket::new();
        payload_vec.extend(HEADER_LEN.to_be_bytes());
        payload_vec.extend(*HEADER_DATA);
        payload_vec.extend(*mac_addr);
        payload_vec.extend(*nonce);
        payload_vec.extend(payload_buf[..padded_tx_len].iter().cloned());
        payload_vec.extend(tag);
        Ok(payload_vec)
    }

    pub fn decrypt(&mut self, aes_packet: AESCCMPacket) -> Result<EspPayload, PacketError> {
        let bytes = aes_packet.inner.as_slice();
        if bytes.len() <= (HEADER_SIZE + NONCE_SIZE + TAG_SIZE) {
            return Err(PacketError::InvalidFormat);
        }
        if bytes[2..2 + HEADER_LEN as usize] != *HEADER_DATA {
            return Err(PacketError::Authentication);
        }
        let mut mac_addr = [0_u8; 8];
        mac_addr[0..8].copy_from_slice(&bytes[HEADER_SIZE..HEADER_SIZE + 8]);
        let mut raw_nonce = [0_u8; 5];
        raw_nonce.copy_from_slice(&bytes[HEADER_SIZE + 8..HEADER_SIZE + NONCE_SIZE]);
        let nonce = u64::from_be_bytes([
            raw_nonce[0],
            raw_nonce[1],
            raw_nonce[2],
            raw_nonce[3],
            raw_nonce[4],
            0,
            0,
            0,
        ]);
        if nonce <= self.rx_nonce.counter {
            return Err(PacketError::Duplicate);
        }
        //tag ========================================================
        let mut tag = [0_u8; 8];
        tag.copy_from_slice(&bytes[bytes.len() - TAG_SIZE..]);
        //a_block ============================================================
        let mut a_block = [0_u8; 16];
        a_block[0] = 0x02;
        a_block[1..=8].copy_from_slice(&mac_addr);
        a_block[9..=13].copy_from_slice(&raw_nonce);
        //
        let payload_len = bytes.len() - HEADER_SIZE - TAG_SIZE - NONCE_SIZE;
        let mut payload_buf = heapless::Vec::<u8, { MAX_PAYLOAD_LEN as usize }>::new();
        let payload_start = HEADER_SIZE + NONCE_SIZE;
        payload_buf.extend(
            bytes[payload_start..payload_start + payload_len]
                .iter()
                .cloned(),
        );
        //inc counter =================================================
        let mut key_stream = [0_u8; 16];
        let (chunks, _) = payload_buf.as_chunks_mut::<16>();
        for chunk in chunks {
            //inc slice as u16
            let mut counter = u16::from_be_bytes([a_block[14], a_block[15]]);
            counter = counter
                .checked_add(1)
                .ok_or(PacketError::AESCounterOverflow)?;
            [a_block[14], a_block[15]] = counter.to_be_bytes();
            //
            key_stream.copy_from_slice(&a_block);
            self.aes.encrypt(&mut key_stream, self.key);
            for j in 0..16 {
                chunk[j] ^= key_stream[j];
            }
        }
        //b_block ============================================================
        let mut b_block = [0_u8; 16];
        b_block[0] = CTL_BYTE;
        b_block[1..=8].copy_from_slice(&mac_addr);
        b_block[9..=13].copy_from_slice(&raw_nonce);
        b_block[14..=15].copy_from_slice(&(payload_len as u16).to_be_bytes());
        let mut head_and_data_payload = [0_u8; HEADER_SIZE + MAX_PAYLOAD_LEN as usize];
        head_and_data_payload[0..=1].copy_from_slice(&HEADER_LEN.to_be_bytes());
        head_and_data_payload[2..HEADER_SIZE].copy_from_slice(HEADER_DATA);
        head_and_data_payload[HEADER_SIZE..HEADER_SIZE + payload_len]
            .copy_from_slice(&payload_buf[..payload_len]);
        //tag gen ============================================================
        self.aes.encrypt(&mut b_block, self.key);
        let (chunks, _) = head_and_data_payload.as_chunks::<16>();
        for chunk in chunks {
            for j in 0..16 {
                b_block[j] ^= chunk[j];
            }
            self.aes.encrypt(&mut b_block, self.key);
        }

        let mut tag_cmp = [0_u8; 8];
        tag_cmp[0..8].copy_from_slice(&b_block[0..8]);
        //tag cmp ====================================================
        a_block[14] = 0;
        a_block[15] = 0;
        key_stream.copy_from_slice(&a_block);
        self.aes.encrypt(&mut key_stream, self.key);
        for i in 0..8 {
            tag_cmp[i] ^= key_stream[i];
        }
        //============================
        if tag != tag_cmp {
            return Err(PacketError::Corrupted);
        }
        let esp_payload = postcard::from_bytes::<EspPayload>(&payload_buf)
            .map_err(|_| PacketError::InvalidFormat)?;
        self.rx_nonce.set(nonce);
        Ok(esp_payload)
    }
}
#[derive(Debug)]
pub struct AESCCMPacket {
    pub inner:
        heapless::Vec<u8, { HEADER_SIZE + MAX_PAYLOAD_LEN as usize + TAG_SIZE + NONCE_SIZE }>,
}
impl AESCCMPacket {
    pub fn new() -> Self {
        Self {
            inner: heapless::Vec::new(),
        }
    }
    pub fn extend<I>(&mut self, iter: I)
    where
        I: IntoIterator<Item = u8>,
    {
        self.inner.extend(iter);
    }
}
