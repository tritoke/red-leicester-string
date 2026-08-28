use base32::Alphabet;

use super::*;

fn base32_decoder(buf: Encoded, meta: DecoderMetadata) -> MaybeDecoded {
    let alphabet = *retrieve_metadata(meta);

    // take the valid UTF8 prefix of this buffer and attempt to base32 decode that...
    let valid_utf8_upto = encoding_rs::Encoding::utf8_valid_up_to(&buf);

    // SAFETY: we are only asking for the prefix of this byte-array that we have checked as being
    // valid UTF8 so it is okay to do this conversion without checks
    let to_decode = unsafe { std::str::from_utf8_unchecked(&buf[..valid_utf8_upto]) };

    base32::decode(alphabet, to_decode).map(Into::into)
}

const BASE32_ALPHABETS: &'static [(Alphabet, DecoderName)] = &[
    (Alphabet::Crockford, "base32-crockford"),
    (Alphabet::Rfc4648 { padding: false }, "base32-RFC-4648"),
    (
        Alphabet::Rfc4648Lower { padding: false },
        "base32-RFC-4648-lower",
    ),
    (
        Alphabet::Rfc4648Hex { padding: false },
        "base32-RFC-4648-hex",
    ),
    (
        Alphabet::Rfc4648HexLower { padding: false },
        "base32-RFC-4648-hex-lower",
    ),
    (Alphabet::Z, "base32-Z"),
];

pub(super) fn base32_codecs(data: &str) -> Vec<Codec> {
    let mut codecs = Vec::with_capacity(BASE32_ALPHABETS.len());
    for (alphabet, decoder_name) in BASE32_ALPHABETS {
        let mut encoded = base32::encode(*alphabet, data.as_bytes());

        if encoded.len() % 7 != 0 {
            encoded.pop();
        }

        codecs.push(Codec {
            encoded: encoded.into_bytes().into(),
            name: decoder_name,
            decoder: base32_decoder,
            metadata: Some(alphabet as &dyn ThreadSafeAny),
        });
    }

    codecs
}
