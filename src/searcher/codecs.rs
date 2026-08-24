// A set of types for tying together the encoding of data and their decoders
pub type Encoded = Box<[u8]>;
pub type Decoded = Option<Box<[u8]>>;
pub type Decoder = fn(Encoded) -> Decoded;
pub type DecoderName = &'static str;
pub type Codec = fn(&[u8]) -> (Encoded, DecoderName, Decoder);

fn identity_decoder(buf: Encoded) -> Decoded {
    Some(buf)
}

fn identity_codec(data: &[u8]) -> (Encoded, &'static str, Decoder) {
    (Box::from(data), "UTF8", identity_decoder)
}

/// Every codec
pub const ALL_CODECS: [Codec; 1] = [identity_codec];
