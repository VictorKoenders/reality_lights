use crate::Result;
use artnet_protocol::ArtCommand;
use bytes::BytesMut;
use failure::Error;
use tokio_codec::{Decoder, Encoder};

#[derive(Default)]
pub struct Codec {}

impl Decoder for Codec {
    type Item = ArtCommand;
    type Error = Error;

    fn decode(&mut self, bytes: &mut BytesMut) -> Result<Option<Self::Item>> {
        Ok(match ArtCommand::from_buffer(&bytes) {
            Ok(c) => Some(c),
            Err(e) => {
                println!("Could not decode bytes: {:?}", e);
                None
            }
        })
    }
}

impl Encoder for Codec {
    type Item = ArtCommand;
    type Error = Error;

    fn encode(&mut self, item: ArtCommand, bytes: &mut BytesMut) -> Result<()> {
        let buffer = match item.into_buffer() {
            Ok(b) => b,
            Err(e) => {
                // Should never happen
                panic!("Could not encode ArtCommand: {:?}", e);
            }
        };
        bytes.extend_from_slice(&buffer);
        Ok(())
    }
}
