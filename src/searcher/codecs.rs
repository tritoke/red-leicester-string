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
    (Box::from(data), "utf8", identity_decoder)
}

macro_rules! base64_codec {
    ($alphabet:path, $decoder:ident, $codec_name:literal, $codec:ident) => {
        fn $decoder(buf: Encoded) -> Decoded {
            const ENGINE: ::base64::engine::general_purpose::GeneralPurpose =
                ::base64::engine::general_purpose::GeneralPurpose::new(
                    &$alphabet,
                    ::base64::engine::general_purpose::NO_PAD_INDIFFERENT,
                );

            if let Ok(decoded) = ::base64::engine::Engine::decode(&ENGINE, &buf) {
                return Some(decoded.into());
            }

            // Try to find a decodeable base64 string by trimming progressively
            for trim_len in (0..buf.len() - 1).rev() {
                // a base64 word can either be AA== AAA= or AAAA
                if trim_len % 4 == 1 {
                    continue;
                }

                if let Ok(decoded) = ::base64::engine::Engine::decode(&ENGINE, &buf[..trim_len]) {
                    return Some(decoded.into());
                }
            }

            None
        }

        fn $codec(data: &[u8]) -> (Encoded, &'static str, Decoder) {
            const ENGINE: ::base64::engine::general_purpose::GeneralPurpose =
                ::base64::engine::general_purpose::GeneralPurpose::new(
                    &$alphabet,
                    ::base64::engine::general_purpose::NO_PAD_INDIFFERENT,
                );

            let mut encoded = ::base64::engine::Engine::encode(&ENGINE, data);
            // if the pattern does not fill the final base64 octet then we must trim it to just the
            // prefix that is fully decided by our flag prefix
            if encoded.len() % 3 != 0 {
                encoded.pop();
            }
            (encoded.into_bytes().into(), $codec_name, $decoder)
        }
    };
}

// While it may seem like adding all of the codecs is excessive, due to the NFA structure of
// the aho-corasick matcher we actually pay very little in terms of size nor speed for the added
// states as most of the alphabets are very close, on some inputs the overhead is literally zero.
base64_codec!(
    base64::alphabet::STANDARD,
    base64_decoder,
    "base64",
    base64_codec
);
base64_codec!(
    base64::alphabet::URL_SAFE,
    base64_urlsafe_decoder,
    "base64-URL-safe",
    base64_urlsafe_codec
);
base64_codec!(
    base64::alphabet::BCRYPT,
    base64_bcrypt_decoder,
    "base64-bcrypt",
    base64_bcrypt_codec
);
base64_codec!(
    base64::alphabet::BIN_HEX,
    base64_binhex_decoder,
    "base64-BinHex",
    base64_binhex_codec
);
base64_codec!(
    base64::alphabet::IMAP_MUTF7,
    base64_imap_mutf7_decoder,
    "base64-IMAP-modified-UTF7",
    base64_imap_mutf7_codec
);

macro_rules! base32_codec {
    ($alphabet:expr, $decoder:ident, $codec_name:literal, $codec:ident) => {
        fn $decoder(buf: Encoded) -> Decoded {
            // take the valid UTF8 prefix of this buffer and attempt to base32 decode that...
            let valid_utf8_upto = ::encoding_rs::Encoding::utf8_valid_up_to(&buf);

            // SAFETY: we are only asking for the prefix of this byte-array that we have checked as being
            // valid UTF8 so it is okay to do this conversion without checks
            let to_decode = unsafe { ::std::str::from_utf8_unchecked(&buf[..valid_utf8_upto]) };

            ::base32::decode($alphabet, to_decode).map(::std::convert::Into::into)
        }

        fn $codec(data: &[u8]) -> (Encoded, &'static str, Decoder) {
            let mut encoded = ::base32::encode($alphabet, data);
            if encoded.len() % 7 != 0 {
                encoded.pop();
            }

            (encoded.into_bytes().into(), $codec_name, $decoder)
        }
    };
}

base32_codec!(
    base32::Alphabet::Crockford,
    base32_crockford_decoder,
    "base32-crockford",
    base32_crockford_codec
);
base32_codec!(
    base32::Alphabet::Rfc4648 { padding: false },
    base32_rfc468_decoder,
    "base32-RFC-4648",
    base32_rfc468_codec
);
base32_codec!(
    base32::Alphabet::Rfc4648Lower { padding: false },
    base32_rfc468_lower_decoder,
    "base32-RFC-4648-lower",
    base32_rfc468_lower_codec
);
base32_codec!(
    base32::Alphabet::Rfc4648Hex { padding: false },
    base32_rfc468_hex_decoder,
    "base32-RFC-4648-hex",
    base32_rfc468_hex_codec
);
base32_codec!(
    base32::Alphabet::Rfc4648HexLower { padding: false },
    base32_rfc468_hex_lower_decoder,
    "base32-RFC-4648-hex-lower",
    base32_rfc468_hex_lower_codec
);
base32_codec!(
    base32::Alphabet::Z,
    base32_z_decoder,
    "base32-Z",
    base32_z_codec
);

/// Every codec
pub const ALL_CODECS: [Codec; 12] = [
    identity_codec,
    // base64 codecs
    base64_codec,
    base64_urlsafe_codec,
    base64_bcrypt_codec,
    base64_binhex_codec,
    base64_imap_mutf7_codec,
    // base32 codecs
    base32_crockford_codec,
    base32_rfc468_codec,
    base32_rfc468_lower_codec,
    base32_rfc468_hex_codec,
    base32_rfc468_hex_lower_codec,
    base32_z_codec,
];

#[cfg(test)]
mod tests {
    use rand::Rng as _;

    use super::*;

    #[test]
    fn all_codecs_roundtrip_to_prefix() {
        let mut random_data = Vec::with_capacity(1000);
        random_data.resize(random_data.capacity(), 0);
        rand::rng().fill_bytes(&mut random_data);
        for codec in ALL_CODECS {
            let (encoded, name, decoder) = codec(&random_data);
            let decoded = &decoder(encoded).unwrap()[..];

            for (dec, cor) in decoded.iter().zip(&random_data) {
                assert_eq!(dec, cor, "Codec {name} roundtrip failed");
            }
        }
    }
}
