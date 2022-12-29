use super::PeerError;
use crate::{
    block::{Block, BlockInfo},
    message::{Handshake, Message, MessageID, PeerCodec},
    Bitfield, PieceIndex,
};
use bytes::{Buf, BufMut};
use tokio::io::Error;
use tokio_util::codec::{Decoder, Encoder};

const PROTOCOL: &str = r"BitTorrent protocol";

pub struct HandShakeCodec;

impl Encoder<Handshake> for HandShakeCodec {
    type Error = Error;

    fn encode(&mut self, item: Handshake, dst: &mut bytes::BytesMut) -> Result<(), Self::Error> {
        assert_eq!(19, PROTOCOL.len());
        dst.put_u8(PROTOCOL.len() as u8);
        dst.put_slice(PROTOCOL.as_bytes());
        dst.put_u64(0);
        dst.put_slice(&item.info_hash);
        dst.put_slice(&item.peer_id);
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
        assert_eq!(pstrlen, 19);

        let mut pstr = vec![0; 19];
        src.copy_to_slice(&mut pstr);
        assert_eq!(pstr, PROTOCOL.as_bytes());

        let mut reserved = vec![0; 8];
        src.copy_to_slice(&mut reserved);

        let mut info_hash =vec![0; 20];
        src.copy_to_slice(&mut info_hash);
        let mut peer_id = vec![0; 20];
        src.copy_to_slice(&mut peer_id);

        assert_eq!(src.remaining(), 0);
        let res = Some(Handshake { info_hash, peer_id, reserved });

        Ok(res)
    }
}

impl Encoder<Message> for PeerCodec {
    type Error = Error;

    fn encode(&mut self, item: Message, dst: &mut bytes::BytesMut) -> Result<(), Self::Error> {
        use Message::*;
        match item {
            KeepAlive => {
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
                let field_length = 1 + bitfield.len() / 8;

                dst.put_u32(field_length as u32);
                dst.put_u8(MessageID::Bitfield as u8);
                dst.extend_from_slice(bitfield.as_raw_slice());
            }
            Request {
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
            Piece {
                index,
                begin,
                block,
            } => {
                dst.put_u32((1 + block.data.len()) as u32);
                dst.put_u8(MessageID::Piece as u8);
                dst.put_u32(index as u32);
                dst.put_u32(begin as u32);
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

        let id = src.get_u8();

        if src.remaining() < len as usize {
            return Ok(None);
        }

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
                let index = src.get_u32() as usize;
                let begin = src.get_u32();
                let length = src.get_u32();
                Message::Request {
                    index,
                    begin,
                    length,
                }
            }
            MessageID::Piece => {
                let data = vec![0; (len - 9) as usize];
                let index = src.get_u32() as usize;
                let begin = src.get_u32();
                let block_info = BlockInfo {
                    piece_index: index,
                    begin,
                };
                let block = Block::new(block_info, data);
                Message::Piece {
                    index,
                    begin,
                    block,
                }
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
