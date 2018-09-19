use artnet_protocol::ArtCommand;
use bytes::BytesMut;
use failure::Error;
use tokio_codec::{Decoder, Encoder};
use Result;

#[derive(Default)]
pub struct Codec {}

impl Decoder for Codec {
    type Item = ArtCommand;
    type Error = Error;

    fn decode(&mut self, bytes: &mut BytesMut) -> Result<Option<Self::Item>> {
        let command = ArtCommand::from_buffer(&bytes).unwrap_or_else(|e| {
            println!("Could not decode bytes: {:?}", e);
            panic!("{:?}", bytes);
        });
        Ok(Some(command))
    }
}

impl Encoder for Codec {
    type Item = ArtCommand;
    type Error = Error;

    fn encode(&mut self, item: ArtCommand, bytes: &mut BytesMut) -> Result<()> {
        let buffer = match item.into_buffer() {
            Ok(b) => b,
            Err(e) => {
                panic!("Could not encode ArtCommand {:?}\n{:?}", item, e);
            }
        };
        bytes.extend_from_slice(&buffer);
        Ok(())
    }
}
