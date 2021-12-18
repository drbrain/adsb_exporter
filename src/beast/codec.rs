use anyhow::Error;

use bytes::Bytes;
use bytes::BytesMut;

use crate::beast::Message;
use crate::beast::Parser;

use nom::Err;

use std::borrow::Borrow;

use tokio_util::codec::Decoder;

pub struct Codec {
    parser: Parser,
}

impl Codec {
    pub fn new() -> Codec {
        let parser = Parser::new();
        Codec { parser }
    }
}

impl Decoder for Codec {
    type Item = Message;
    type Error = Error;

    fn decode(&mut self, buf: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        let bytes = buf.split_to(buf.len());
        let input = bytes.borrow();

        match self.parser.parse(input) {
            Ok((input, message)) => {
                buf.extend_from_slice(&Bytes::copy_from_slice(input));

                Ok(Some(message))
            }
            Err(Err::Incomplete(_)) => {
                buf.extend_from_slice(&Bytes::copy_from_slice(input));

                Ok(None)
            }
            Err(Err::Error(e)) => panic!("impossible error! {:?}", e),
            Err(Err::Failure(_)) => panic!("impossible failure!"),
        }
    }
}
