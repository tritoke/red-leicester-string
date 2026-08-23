use std::borrow::Cow;

// TODO: Maybe replace with a trait + empty struct implementing the trait so we can
// recover the name of the codec used after the fact

// A set of types for tying together the encoding of data and their decoders
pub type Decoder = for<'a> fn(Cow<'a, [u8]>) -> Option<Cow<'a, [u8]>>;
pub type Encoded = Box<[u8]>;
pub type Decoded<'a> = Option<Cow<'a, [u8]>>;
pub type DecoderName = &'static str;
pub type Codec = fn(&[u8]) -> (Encoded, DecoderName, Decoder);

fn identity_decoder<'a>(buf: Cow<'a, [u8]>) -> Decoded<'a> {
    Some(buf)
}

fn identity_codec(data: &[u8]) -> (Encoded, &'static str, Decoder) {
    (Box::from(data), "UTF8", identity_decoder)
}

/// Every codec
pub const ALL_CODECS: [Codec; 1] = [identity_codec];
