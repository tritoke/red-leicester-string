use std::borrow::Cow;

// TODO: Maybe replace with a trait + empty struct implementing the trait so we can
// recover the name of the codec used after the fact

// A set of types for tying together the encoding of data and their decoders
pub type Decoder = for<'a> fn(&'a [u8]) -> Option<Cow<'a, [u8]>>;
pub type Encoded = Box<[u8]>;
pub type Codec = fn(&[u8]) -> (Encoded, Decoder);

fn identity_decoder<'a>(buf: &'a [u8]) -> Option<Cow<'a, [u8]>> {
    Some(Cow::Borrowed(buf))
}

pub fn identity_codec(data: &[u8]) -> (Encoded, Decoder) {
    (Box::from(data), identity_decoder)
}

/// Every codec
pub const ALL_CODECS: [Codec; 1] = [identity_codec];
