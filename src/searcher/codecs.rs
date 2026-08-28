use std::any::Any;

use base64::Engine as _;

mod base32_codecs;
mod base64_codecs;
mod common_encodings;
mod ctf_codecs;
mod encoding_rs_codecs;

// A set of types for tying together the encoding of data and their decoders
pub type Encoded = Box<[u8]>;
pub type MaybeDecoded = Option<Box<[u8]>>;

// Any but we can share it safely
pub trait ThreadSafeAny: Any + Send + Sync {}
impl<T> ThreadSafeAny for T where T: Any + Send + Sync {}

// type erase metadata so all of them maintain the same type
pub type DecoderMetadata = Option<&'static dyn ThreadSafeAny>;
pub type DecoderName = &'static str;
pub struct Codec {
    pub encoded: Encoded,
    pub name: &'static str,
    pub decoder: fn(Encoded, DecoderMetadata) -> MaybeDecoded,
    pub metadata: DecoderMetadata,
}
pub type CodecGenerator = fn(&str) -> Vec<Codec>;

/// Force downcasting the metadata to the specified type, panics if the metadata is missing or if it
/// is of the wrong type
fn retrieve_metadata<T>(meta: DecoderMetadata) -> &'static T {
    let meta_ref = meta.expect("metadata was missing");
    (meta_ref as &dyn Any)
        .downcast_ref()
        .expect("metadata was the wrong type")
}

// NOTE: matches for these are returned in the order they are defined here so less likely / weirder
// codecs should be put further down
pub const ALL_CODEC_GENERATORS: &'static [CodecGenerator] = &[
    common_encodings::common_encodings,
    base64_codecs::base64_codecs,
    base32_codecs::base32_codecs,
    ctf_codecs::ctf_codecs,
    encoding_rs_codecs::encoding_rs_codecs,
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_codecs_roundtrip_to_prefix() {
        let mut random_data = Vec::with_capacity(999);
        random_data.resize(random_data.capacity(), 0);
        for b in random_data.iter_mut() {
            *b = rand::random_range(b' '..=b'~');
        }
        // all data is valid ascii so this is fine
        let random_str = unsafe { std::str::from_utf8_unchecked(&random_data) };

        // if we are running in release mode include every codec
        if !cfg!(debug_assertions) {
            crate::GAMBLE.store(true, std::sync::atomic::Ordering::Relaxed);
        }

        for codec_generator in ALL_CODEC_GENERATORS {
            for codec in codec_generator(random_str) {
                let Codec {
                    encoded,
                    name,
                    decoder,
                    metadata,
                } = codec;
                let decoded = &decoder(encoded.clone(), metadata).unwrap()[..];
                for (dec, cor) in decoded.iter().zip(&random_data) {
                    assert_eq!(dec, cor, "Codec {} roundtrip failed", name);
                }
            }
        }
    }
}
