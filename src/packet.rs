use anyhow::Result;
use byteorder::{BigEndian, ByteOrder};
use proto::{CheshireMessage, ProtoMsg};
use std::fmt::Write;

#[derive(thiserror::Error, Debug)]
enum PacketError {
    #[error("packet too short")]
    TooShort,
    #[error("invalid packet length")]
    InvalidLength,
    #[error("packet length mismatch")]
    LenMismatch,
}

#[derive(Debug, Clone)]
pub struct Packet {
    pub length: u16,
    pub flag: u8,
    pub cmd_id: u16,
    pub id: u16,
    pub data: Vec<u8>,
    pub raw: Vec<u8>,
}

impl Packet {
    const LEN_SIZE: usize = 2;
    const HEADER_SIZE: usize = 5;
    const FRAME_HEADER_SIZE: usize = Self::LEN_SIZE + Self::HEADER_SIZE;

    pub fn new(data: Vec<u8>) -> Result<Self> {
        if data.len() < Self::FRAME_HEADER_SIZE {
            return Err(PacketError::TooShort.into());
        }

        let length = BigEndian::read_u16(&data[..Self::LEN_SIZE]);
        if length as usize != Self::HEADER_SIZE + 1 && (length as usize) < Self::HEADER_SIZE {
            return Err(PacketError::InvalidLength.into());
        }

        let expected = length as usize + Self::LEN_SIZE;
        let empty_frame_expected = Self::FRAME_HEADER_SIZE;
        if data.len() != expected
            && !(length as usize == Self::HEADER_SIZE + 1 && data.len() == empty_frame_expected)
        {
            return Err(PacketError::LenMismatch.into());
        }

        let flag = data[Self::LEN_SIZE];
        let cmd_id = BigEndian::read_u16(&data[3..5]);
        let id = BigEndian::read_u16(&data[5..7]);
        let payload_len =
            if length as usize == Self::HEADER_SIZE + 1 && data.len() == empty_frame_expected {
                0
            } else {
                length as usize - Self::HEADER_SIZE
            };
        let payload = data[7..7 + payload_len].to_vec();

        Ok(Self {
            length,
            flag,
            cmd_id,
            id,
            data: payload,
            raw: data,
        })
    }

    pub fn encode<T: CheshireMessage>(message: &T, id: u16) -> Self {
        Self::encode_raw(message.get_cmd_id(), id, message.encode_to_vec())
    }

    pub fn encode_raw(cmd_id: u16, id: u16, data: Vec<u8>) -> Self {
        Self {
            length: (Self::HEADER_SIZE + data.len()) as u16,
            flag: 0,
            cmd_id,
            id,
            data,
            raw: Vec::new(),
        }
    }

    pub fn decode<T: ProtoMsg + Default>(&self) -> Option<T> {
        T::decode(self.data.as_slice()).ok()
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(self.length as usize + Self::LEN_SIZE);
        let mut buf = [0u8; 2];
        BigEndian::write_u16(&mut buf, self.length);
        out.extend_from_slice(&buf);
        out.push(self.flag);
        BigEndian::write_u16(&mut buf, self.cmd_id);
        out.extend_from_slice(&buf);
        BigEndian::write_u16(&mut buf, self.id);
        out.extend_from_slice(&buf);
        out.extend_from_slice(&self.data);
        out
    }

    pub fn raw_len(&self) -> usize {
        if self.raw.is_empty() {
            self.length as usize + Self::LEN_SIZE
        } else {
            self.raw.len()
        }
    }

    pub fn raw_hex_prefix(&self, max: usize) -> String {
        if self.raw.is_empty() {
            return hex_prefix(&self.to_bytes(), max);
        }

        hex_prefix(&self.raw, max)
    }

    pub fn split_from(buffer: &mut Vec<u8>) -> Option<Result<Self>> {
        Self::split_from_after(buffer, None)
    }

    pub fn split_from_after(
        buffer: &mut Vec<u8>,
        previous_id: Option<u16>,
    ) -> Option<Result<Self>> {
        Self::drop_padding_before_next_packet(buffer, previous_id);

        if buffer.len() < Self::LEN_SIZE {
            return None;
        }

        let length = BigEndian::read_u16(&buffer[..Self::LEN_SIZE]) as usize;
        let total = if length == Self::HEADER_SIZE + 1 {
            Self::FRAME_HEADER_SIZE
        } else {
            length + Self::LEN_SIZE
        };
        if buffer.len() < total {
            return None;
        }

        let raw = buffer.drain(..total).collect();
        Some(Self::new(raw))
    }

    fn drop_padding_before_next_packet(buffer: &mut Vec<u8>, previous_id: Option<u16>) {
        let Some(previous_id) = previous_id else {
            return;
        };

        if !Self::looks_like_next_packet(buffer, 0, previous_id)
            && Self::looks_like_next_packet(buffer, 1, previous_id)
        {
            let dropped = buffer.remove(0);
            tracing::warn!(
                previous_packet_id = previous_id,
                dropped,
                "dropped packet padding byte before next client packet"
            );
        }
    }

    fn looks_like_next_packet(buffer: &[u8], offset: usize, previous_id: u16) -> bool {
        if buffer.len().saturating_sub(offset) < Self::FRAME_HEADER_SIZE {
            return false;
        }

        let length = BigEndian::read_u16(&buffer[offset..offset + Self::LEN_SIZE]) as usize;
        let flag = buffer[offset + Self::LEN_SIZE];
        let cmd_id = BigEndian::read_u16(&buffer[offset + 3..offset + 5]);
        let packet_id = BigEndian::read_u16(&buffer[offset + 5..offset + 7]);

        length >= Self::HEADER_SIZE
            && flag == 0
            && cmd_id >= 10_000
            && packet_id == previous_id.wrapping_add(1)
    }
}

fn hex_prefix(bytes: &[u8], max: usize) -> String {
    let mut out = String::new();
    for (index, byte) in bytes.iter().take(max).enumerate() {
        if index != 0 {
            out.push(' ');
        }
        let _ = write!(&mut out, "{byte:02x}");
    }
    if bytes.len() > max {
        let _ = write!(&mut out, " ...(+{} bytes)", bytes.len() - max);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::Packet;
    use proto::p63::Sc63318;

    #[test]
    fn empty_payload_uses_wire_length_without_padding() {
        let packet = Packet::encode(&Sc63318::default(), 7);

        assert_eq!(packet.length, 5);
        assert_eq!(packet.to_bytes(), vec![0, 5, 0, 247, 86, 0, 7]);
    }

    #[test]
    fn parses_empty_client_frame_without_waiting_for_padding() {
        let mut buffer = vec![0, 6, 0, 43, 9, 0, 3, 0, 6, 0, 39, 17, 0, 4];
        let first = Packet::split_from(&mut buffer).unwrap().unwrap();
        let second = Packet::split_from(&mut buffer).unwrap().unwrap();

        assert_eq!(first.cmd_id, 11017);
        assert_eq!(first.id, 3);
        assert!(first.data.is_empty());
        assert_eq!(second.cmd_id, 10001);
        assert_eq!(second.id, 4);
        assert!(second.data.is_empty());
        assert!(buffer.is_empty());
    }

    #[test]
    fn drops_delayed_padding_byte_before_next_ordered_packet() {
        let mut buffer = vec![0, 6, 0, 0x3a, 0xa0, 0, 22];
        let first = Packet::split_from_after(&mut buffer, Some(21))
            .unwrap()
            .unwrap();

        assert_eq!(first.cmd_id, 15008);
        assert_eq!(first.id, 22);
        assert_eq!(first.raw_len(), 7);
        assert!(first.data.is_empty());
        assert!(buffer.is_empty());

        buffer.extend_from_slice(&[1, 0, 7, 0, 0x2d, 0xca, 0, 23, 8, 93]);
        let second = Packet::split_from_after(&mut buffer, Some(first.id))
            .unwrap()
            .unwrap();

        assert_eq!(second.cmd_id, 11722);
        assert_eq!(second.id, 23);
        assert_eq!(second.data, vec![8, 93]);
        assert_eq!(second.raw_hex_prefix(16), "00 07 00 2d ca 00 17 08 5d");
        assert!(buffer.is_empty());
    }
}
