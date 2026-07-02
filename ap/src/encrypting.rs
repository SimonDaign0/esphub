pub struct AesHal(pub esp_hal::aes::Aes<'static>);
use mcu_comms::aesccm::Encrypt;

impl Encrypt for AesHal {
    fn encrypt(&mut self, key_stream_buf: &mut [u8; 16], block: &mut [u8; 16], key: [u8; 16]) {
        key_stream_buf.copy_from_slice(block);
        self.0.encrypt(key_stream_buf, key);
    }
}
