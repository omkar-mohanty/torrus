use super::PeerError;
use crate::{
    block::{Block, BlockInfo, BLOCK_SIZE},
    message::{Handshake, Message, MessageID},
    Bitfield, PieceIndex,
};
use bytes::{Buf, BufMut};
use tokio::io::Error;
use tokio_util::codec::{Decoder, Encoder};

const PROTOCOL: &str = "BitTorrent protocol";

pub struct HandShakeCodec;
pub struct PeerCodec;

impl Encoder<Handshake> for HandShakeCodec {
    type Error = Error;

    fn encode(&mut self, item: Handshake, dst: &mut bytes::BytesMut) -> Result<(), Self::Error> {
        assert_eq!(19, PROTOCOL.len());
        dst.put_u8(PROTOCOL.len() as u8);
        dst.extend_from_slice(PROTOCOL.as_bytes());
        dst.extend_from_slice(&item.reserved);
        dst.extend_from_slice(&item.info_hash);
        dst.extend_from_slice(&item.peer_id);
        Ok(())
    }
}

impl Decoder for HandShakeCodec {
    type Error = PeerError;
    type Item = Handshake;

    fn decode(&mut self, src: &mut bytes::BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        if src.len() < Handshake::len() {
            return Ok(None);
        }

        let pstrlen = src.get_u8();

        if pstrlen != 19 {
            let err_msg = format!(
                "The length of the protocol identifier must be 19 but it is {}",
                pstrlen
            );
            return Err(PeerError::new(&err_msg));
        }

        let mut pstr = [0; 19];
        src.copy_to_slice(&mut pstr);

        let mut reserved = [0; 8];
        src.copy_to_slice(&mut reserved);

        let mut info_hash = vec![0_u8; 20];
        src.copy_to_slice(&mut info_hash);
        let mut peer_id = [0; 20];
        src.copy_to_slice(&mut peer_id);

        let res = Some(Handshake {
            info_hash,
            peer_id,
            reserved,
        });

        Ok(res)
    }
}

impl Encoder<Message> for PeerCodec {
    type Error = Error;

    /// The messages described as per Bittorrent protocol is <length prefix><message ID>
    /// length prefix : 4 bytes
    /// message ID : 1 byte
    fn encode(&mut self, item: Message, dst: &mut bytes::BytesMut) -> Result<(), Self::Error> {
        use Message::*;
        match item {
            KeepAlive => {
                // KeepAlive message length prefix is always 0
                dst.put_u32(0);
            }
            Choke => {
                dst.put_u32(1);
                dst.put_u8(MessageID::Choke as u8);
            }
            Unchoke => {
                dst.put_u32(1);
                dst.put_u8(MessageID::Unchoke as u8);
            }
            Interested => {
                dst.put_u32(1);
                dst.put_u8(MessageID::Interested as u8);
            }
            NotInterested => {
                dst.put_u32(1);
                dst.put_u8(MessageID::NotInterested as u8);
            }
            Have(index) => {
                dst.put_u32(5);
                dst.put_u8(MessageID::Have as u8);
                dst.put_u32(index as u32);
            }
            Bitfield(bitfield) => {
                // 'biffield.len()' gives total number of bits but we want length in bytes.
                let field_length = 1 + bitfield.len() / 8;

                dst.put_u32(field_length as u32);
                dst.put_u8(MessageID::Bitfield as u8);
                dst.extend_from_slice(bitfield.as_raw_slice());
            }
            Request(block_info) => {
                dst.put_u32(13);
                dst.put_u8(MessageID::Request as u8);
                dst.put_u32(block_info.piece_index as u32);
                dst.put_u32(block_info.begin);
                dst.put_u32(block_info.length);
            }
            Piece(block) => {
                // 1 byte Message ID + 4 bytes piece index + 4 bytes offset = 9 bytes fixed
                dst.put_u32((9 + block.data.len()) as u32);
                dst.put_u8(MessageID::Piece as u8);
                dst.put_u32(block.block_info.piece_index as u32);
                dst.put_u32(block.block_info.begin);
                dst.extend_from_slice(&block);
            }
            Cancel {
                index,
                begin,
                length,
            } => {
                dst.put_u32(13);
                dst.put_u8(MessageID::Request as u8);
                dst.put_u32(index as u32);
                dst.put_u32(begin);
                dst.put_u32(length);
            }
            Port(listen_port) => {
                dst.put_u32(3);
                dst.put_u8(9);
                dst.put_u16(listen_port);
            }
        }

        Ok(())
    }
}

impl Decoder for PeerCodec {
    type Error = PeerError;
    type Item = Message;

    fn decode(&mut self, src: &mut bytes::BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        if src.remaining() < 4 {
            return Ok(None);
        }

        let len = src.get_u32();
        if len == 0 {
            return Ok(Some(Message::KeepAlive));
        }

        if src.remaining() < len as usize {
            return Ok(None);
        }

        let id = src.get_u8();

        let message_id = MessageID::try_from(id)?;

        let msg = match message_id {
            MessageID::Choke => Message::Choke,
            MessageID::Unchoke => Message::Unchoke,
            MessageID::Interested => Message::Interested,
            MessageID::NotInterested => Message::NotInterested,
            MessageID::Have => {
                let piece_index: PieceIndex = src.get_u32() as usize;

                Message::Have(piece_index)
            }
            MessageID::Bitfield => {
                let mut bitfield = vec![0; (len - 1) as usize];
                src.copy_to_slice(&mut bitfield);
                Message::Bitfield(Bitfield::from_vec(bitfield))
            }
            MessageID::Request => {
                let piece_index = src.get_u32() as usize;
                let begin = src.get_u32();
                let length = src.get_u32();
                let block_info = BlockInfo {
                    piece_index,
                    begin,
                    length,
                };
                Message::Request(block_info)
            }
            MessageID::Piece => {
                // 9 bytes is fixed in all 'Piece' messages the actual paylod length is always
                // data_len = len - 9.
                if len - 9 > BLOCK_SIZE {
                    return Err(PeerError::new(
                        "The length of the BLOCK exceeds maximum allowed block size",
                    ));
                }
                let mut data = vec![0; (len - 9) as usize];
                let index = src.get_u32() as usize;
                let begin = src.get_u32();
                src.copy_to_slice(&mut data);
                let block_info = BlockInfo {
                    piece_index: index,
                    begin,
                    length: len - 9,
                };
                let block = Block::new(block_info, data);
                Message::Piece(block)
            }
            MessageID::Cancel => {
                let index = src.get_u32() as usize;
                let begin = src.get_u32();
                let length = src.get_u32();

                Message::Cancel {
                    index,
                    begin,
                    length,
                }
            }
            MessageID::Port => {
                let listen_port = src.get_u16();
                Message::Port(listen_port)
            }
        };

        Ok(Some(msg))
    }
}

#[cfg(test)]
mod tests {
    const BLOCK_INFO: BlockInfo = BlockInfo {
        piece_index: 12,
        length: 12,
        begin: 12,
    };
    use super::*;
    use crate::{block::BLOCK_SIZE, Result};
    use bytes::BytesMut;
    use rand::{thread_rng, Rng};

    fn fixed_len_message(id: u8) -> Message {
        match id {
            0 => Message::KeepAlive,
            1 => Message::Choke,
            2 => Message::Unchoke,
            3 => Message::Interested,
            4 => Message::NotInterested,
            5 => Message::Have(12),
            6 => Message::Request(BLOCK_INFO),
            7 => Message::Cancel {
                index: 12,
                begin: 12,
                length: 12,
            },
            8 => Message::Port(8080),
            _ => Message::KeepAlive,
        }
    }

    fn correct_handshake() -> Handshake {
        let peer_id = thread_rng().gen::<[u8; 20]>();
        let hash = thread_rng().gen::<[u8; 20]>().to_vec();
        Handshake::new(peer_id, hash)
    }

    fn incorrect_handshake() -> BytesMut {
        let peer_id = thread_rng().gen::<[u8; 20]>();
        let hash = thread_rng().gen::<[u8; 21]>().to_vec();
        let handshake = Handshake::new(peer_id, hash);
        let mut dst = BytesMut::new();
        HandShakeCodec.encode(handshake, &mut dst).unwrap();
        dst
    }

    #[tokio::test]
    async fn test_handshake() -> Result<()> {
        let handshake = correct_handshake();
        let mut dst = BytesMut::new();
        HandShakeCodec.encode(handshake, &mut dst)?;

        println!("{}", dst.len());
        assert_eq!(
            dst.len(),
            Handshake::len(),
            "The length of the handshake after encoding must be {} bytes long",
            Handshake::len()
        );

        HandShakeCodec.decode(&mut dst)?.unwrap();
        Ok(())
    }

    #[tokio::test]
    async fn test_incorrect_handshake() {
        let mut dst = incorrect_handshake();
        HandShakeCodec.decode(&mut dst).unwrap();
    }

    /// After decoding the buffer must have zero bytes remaining.
    #[test]
    fn test_message_codec() -> Result<()> {
        for id in 0..=8 {
            let msg = fixed_len_message(id);
            let mut dst = BytesMut::new();

            PeerCodec.encode(msg, &mut dst)?;
            let msg = PeerCodec.decode(&mut dst)?.unwrap();

            assert_eq!(
                dst.remaining(),
                0,
                "Number of bytes in buffer not zero for message : {}",
                msg
            );
        }
        Ok(())
    }

    #[test]
    fn test_bitfield_codec() -> Result<()> {
        let bitfield = Bitfield::new();

        let msg = Message::Bitfield(bitfield);
        let mut dst = BytesMut::new();

        PeerCodec.encode(msg, &mut dst)?;
        let msg = PeerCodec.decode(&mut dst)?.unwrap();

        matches!(msg, Message::Bitfield(_));
        assert_eq!(
            dst.remaining(),
            0,
            "After decoding the remaining bytes must be 0"
        );
        Ok(())
    }

    #[test]
    fn test_piece_codec() -> Result<()> {
        let data_len = thread_rng().gen_range(0..BLOCK_SIZE);
        let mut data = Vec::new();

        for _ in 0..=data_len {
            data.push(thread_rng().gen::<u8>());
        }

        let block = Block::new(BLOCK_INFO, data);
        let mut dst = BytesMut::new();
        let msg = Message::Piece(block);
        PeerCodec.encode(msg, &mut dst)?;

        let msg = PeerCodec.decode(&mut dst)?.unwrap();

        matches!(msg, Message::Piece { .. });
        assert_eq!(dst.remaining(), 0);
        Ok(())
    }

    #[test]
    #[should_panic]
    fn test_incorrect_block_size() {
        let block_info = BlockInfo::new(12, 12);

        let data_len = BLOCK_SIZE + 1;
        let mut data = Vec::new();

        for _ in 0..=data_len {
            data.push(thread_rng().gen::<u8>());
        }

        let block = Block::new(block_info, data);
        let mut dst = BytesMut::new();
        let msg = Message::Piece(block);
        PeerCodec.encode(msg, &mut dst).unwrap();

        let msg = PeerCodec.decode(&mut dst).unwrap().unwrap();
        matches!(msg, Message::Piece { .. });

        assert_eq!(dst.remaining(), 0);
    }
}
