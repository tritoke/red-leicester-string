use std::any::Any;

use base64::Engine as _;

// A set of types for tying together the encoding of data and their decoders
pub type Encoded = Box<[u8]>;
pub type Decoded = Option<Box<[u8]>>;

// Any but we can share it safely
pub trait ThreadSafeAny: Any + Send + Sync {}
impl<T> ThreadSafeAny for T where T: Any + Send + Sync {}

// type erase metadata so all of them maintain the same type
pub type DecoderMetadata = Option<&'static dyn ThreadSafeAny>;
pub type DecoderName = &'static str;
pub struct Codec {
    pub encoded: Encoded,
    pub name: &'static str,
    pub decoder: fn(Encoded, DecoderMetadata) -> Decoded,
    pub metadata: DecoderMetadata,
}
pub type CodecGenerator = fn(&str) -> Vec<Codec>;

fn identity_decoder(buf: Encoded, _meta: DecoderMetadata) -> Decoded {
    Some(buf)
}

fn identity_codec(data: &str) -> Vec<Codec> {
    vec![Codec {
        encoded: Box::from(data.as_bytes()),
        name: "UTF8",
        decoder: identity_decoder,
        metadata: None,
    }]
}

/// Force downcasting the metadata to the specified type, panics if the metadata is missing or if it
/// is of the wrong type
fn retrieve_metadata<T>(meta: DecoderMetadata) -> &'static T {
    let meta_ref = meta.expect("metadata was missing");
    (meta_ref as &dyn Any)
        .downcast_ref()
        .expect("metadata was the wrong type")
}

mod base64_codecs {
    use super::*;

    const BCRYPT_NO_PAD_INDIFFERENT: base64::engine::GeneralPurpose =
        base64::engine::GeneralPurpose::new(
            &base64::alphabet::BCRYPT,
            ::base64::engine::general_purpose::NO_PAD_INDIFFERENT,
        );

    const BIN_HEX_NO_PAD_INDIFFERENT: base64::engine::GeneralPurpose =
        base64::engine::GeneralPurpose::new(
            &base64::alphabet::BIN_HEX,
            ::base64::engine::general_purpose::NO_PAD_INDIFFERENT,
        );

    const CRYPT_NO_PAD_INDIFFERENT: base64::engine::GeneralPurpose =
        base64::engine::GeneralPurpose::new(
            &base64::alphabet::CRYPT,
            base64::engine::general_purpose::NO_PAD_INDIFFERENT,
        );

    const IMAP_MUTF7_NO_PAD_INDIFFERENT: base64::engine::GeneralPurpose =
        base64::engine::GeneralPurpose::new(
            &base64::alphabet::IMAP_MUTF7,
            ::base64::engine::general_purpose::NO_PAD_INDIFFERENT,
        );

    const BASE64_ENGINES: [(&'static base64::engine::GeneralPurpose, DecoderName); 6] = [
        (
            &base64::engine::general_purpose::STANDARD_NO_PAD_INDIFFERENT,
            "base64",
        ),
        (
            &base64::engine::general_purpose::URL_SAFE_NO_PAD_INDIFFERENT,
            "base64-URL-safe",
        ),
        (&BCRYPT_NO_PAD_INDIFFERENT, "base64-bcrypt"),
        (&BIN_HEX_NO_PAD_INDIFFERENT, "base64-bin-hex"),
        (&CRYPT_NO_PAD_INDIFFERENT, "base64-crypt"),
        (&IMAP_MUTF7_NO_PAD_INDIFFERENT, "base64-IMAP-modified-UTF7"),
    ];

    fn base64_decoder(buf: Encoded, meta: DecoderMetadata) -> Decoded {
        let engine: &&base64::engine::GeneralPurpose = retrieve_metadata(meta);

        if let Ok(decoded) = engine.decode(&buf) {
            return Some(decoded.into());
        }

        // Try to find a decodeable base64 string by trimming progressively
        for trim_len in (0..buf.len() - 1).rev() {
            // a base64 word can either be AA== AAA= or AAAA
            if trim_len % 4 == 1 {
                continue;
            }

            if let Ok(decoded) = engine.decode(&buf[..trim_len]) {
                return Some(decoded.into());
            }
        }

        None
    }

    pub(super) fn base64_codecs(data: &str) -> Vec<Codec> {
        let mut codecs = Vec::with_capacity(BASE64_ENGINES.len());
        for (engine, decoder_name) in BASE64_ENGINES {
            let mut encoded = engine.encode(data.as_bytes());
            // if the pattern does not fill the final base64 octet then we must trim it to just the
            // prefix that is fully decided by our flag prefix
            if encoded.len() % 3 != 0 {
                encoded.pop();
            }
            codecs.push(Codec {
                encoded: encoded.into_bytes().into(),
                name: decoder_name,
                decoder: base64_decoder,
                metadata: Some(Box::leak(Box::new(engine))),
            });
        }

        codecs
    }
}

use base64_codecs::base64_codecs;

// macro_rules! base32_codec {
//     ($alphabet:expr, $decoder:ident, $codec_name:literal, $codec:ident) => {
//         fn $decoder(buf: Encoded) -> Decoded {
//             // take the valid UTF8 prefix of this buffer and attempt to base32 decode that...
//             let valid_utf8_upto = ::encoding_rs::Encoding::utf8_valid_up_to(&buf);
//
//             // SAFETY: we are only asking for the prefix of this byte-array that we have checked as being
//             // valid UTF8 so it is okay to do this conversion without checks
//             let to_decode = unsafe { ::std::str::from_utf8_unchecked(&buf[..valid_utf8_upto]) };
//
//             ::base32::decode($alphabet, to_decode).map(::std::convert::Into::into)
//         }
//
//         fn $codec(data: &str) -> (Encoded, DecoderName, Decoder) {
//             let mut encoded = ::base32::encode($alphabet, data.as_bytes());
//             if encoded.len() % 7 != 0 {
//                 encoded.pop();
//             }
//
//             (encoded.into_bytes().into(), $codec_name, $decoder)
//         }
//     };
// }
//
// base32_codec!(
//     base32::Alphabet::Crockford,
//     base32_crockford_decoder,
//     "base32-crockford",
//     base32_crockford_codec
// );
// base32_codec!(
//     base32::Alphabet::Rfc4648 { padding: false },
//     base32_rfc468_decoder,
//     "base32-RFC-4648",
//     base32_rfc468_codec
// );
// base32_codec!(
//     base32::Alphabet::Rfc4648Lower { padding: false },
//     base32_rfc468_lower_decoder,
//     "base32-RFC-4648-lower",
//     base32_rfc468_lower_codec
// );
// base32_codec!(
//     base32::Alphabet::Rfc4648Hex { padding: false },
//     base32_rfc468_hex_decoder,
//     "base32-RFC-4648-hex",
//     base32_rfc468_hex_codec
// );
// base32_codec!(
//     base32::Alphabet::Rfc4648HexLower { padding: false },
//     base32_rfc468_hex_lower_decoder,
//     "base32-RFC-4648-hex-lower",
//     base32_rfc468_hex_lower_codec
// );
// base32_codec!(
//     base32::Alphabet::Z,
//     base32_z_decoder,
//     "base32-Z",
//     base32_z_codec
// );
//
// fn utf16_le_decoder(buf: Encoded) -> Decoded {
//     let decoded = String::from_utf16le_lossy(&buf);
//     Some(decoded.into_bytes().into())
// }
//
// fn utf16_le_codec(data: &str) -> (Encoded, DecoderName, Decoder) {
//     let utf16_codepoints = data.encode_utf16();
//     let utf16_bytes: Vec<u8> = utf16_codepoints
//         .flat_map(|codepoint| codepoint.to_le_bytes())
//         .collect();
//
//     (utf16_bytes.into(), "UTF-16-LE", utf16_le_decoder)
// }
//
// fn utf16_be_decoder(buf: Encoded) -> Decoded {
//     let decoded = String::from_utf16be_lossy(&buf);
//     Some(decoded.into_bytes().into())
// }
//
// fn utf16_be_codec(data: &str) -> (Encoded, DecoderName, Decoder) {
//     let utf16_codepoints = data.encode_utf16();
//     let utf16_bytes: Vec<u8> = utf16_codepoints
//         .flat_map(|codepoint| codepoint.to_be_bytes())
//         .collect();
//
//     (utf16_bytes.into(), "UTF-16-BE", utf16_be_decoder)
// }
//
// fn utf32_le_decoder(buf: Encoded) -> Decoded {
//     let (utf32_byte_pairs, _trailing): (&[[u8; 4]], &[u8]) = buf.as_chunks();
//     let decoded: String = utf32_byte_pairs
//         .iter()
//         .map(|codepoint| {
//             char::from_u32(u32::from_le_bytes(*codepoint)).unwrap_or(char::REPLACEMENT_CHARACTER)
//         })
//         .collect();
//
//     Some(decoded.into_bytes().into())
// }
//
// fn utf32_le_codec(data: &str) -> (Encoded, DecoderName, Decoder) {
//     let utf32_codepoints = data.chars().map(|c| c as u32);
//     let utf32_bytes: Vec<u8> = utf32_codepoints
//         .flat_map(|codepoint| codepoint.to_le_bytes())
//         .collect();
//
//     (utf32_bytes.into(), "UTF-32-LE", utf32_le_decoder)
// }
//
// fn utf32_be_decoder(buf: Encoded) -> Decoded {
//     let (utf32_byte_pairs, _trailing): (&[[u8; 4]], &[u8]) = buf.as_chunks();
//     let decoded: String = utf32_byte_pairs
//         .iter()
//         .map(|codepoint| {
//             char::from_u32(u32::from_be_bytes(*codepoint)).unwrap_or(char::REPLACEMENT_CHARACTER)
//         })
//         .collect();
//
//     Some(decoded.into_bytes().into())
// }
//
// fn utf32_be_codec(data: &str) -> (Encoded, DecoderName, Decoder) {
//     let utf32_codepoints = data.chars().map(|c| c as u32);
//     let utf32_bytes: Vec<u8> = utf32_codepoints
//         .flat_map(|codepoint| codepoint.to_be_bytes())
//         .collect();
//
//     (utf32_bytes.into(), "UTF-32-BE", utf32_be_decoder)
// }
//
// fn base10_ascii_decoder(buf: Encoded) -> Decoded {
//     // this is a strict upper bound from the length of buf
//     let mut acc = String::with_capacity(buf.len() / 2);
//
//     let mut cur = 0;
//     for c in buf {
//         if !c.is_ascii_digit() {
//             break;
//         }
//
//         cur *= 10;
//         cur += c - b'0';
//
//         if cur >= b' ' {
//             acc.push(cur as char);
//             cur = 0;
//         }
//     }
//
//     Some(acc.into_bytes().into())
// }
//
// fn base10_ascii_codec(data: &str) -> (Encoded, DecoderName, Decoder) {
//     use std::io::Write;
//
//     let mut buf = Vec::with_capacity(data.chars().count() * 3);
//     for c in data.chars() {
//         write!(&mut buf, "{}", c as u32).unwrap();
//     }
//
//     (buf.into(), "base10-ascii", base10_ascii_decoder)
// }
//
// macro_rules! encoding_rs_codec {
//     ($encoding:path, $decoder:ident, $codec_name:literal, $codec:ident) => {
//         fn $decoder(buf: Encoded) -> Decoded {
//             let (decoded, _, _) = $encoding.decode(buf.as_ref());
//             // Cow -> String -> Vec<[u8]> -> [u8]
//             Some(decoded.into_owned().into_bytes().into())
//         }
//
//         pub fn $codec(data: &str) -> (Encoded, DecoderName, Decoder) {
//             let (encoded, _, _) = $encoding.encode(data);
//             (encoded.into_owned().into(), $codec_name, $decoder)
//         }
//     };
// }
//
// // put these in a module so I can format them nicely
// #[rustfmt::skip]
// mod encoding_rs_codecs {
//     use super::*;
//
//     encoding_rs_codec!(encoding_rs::BIG5,           big5_decoder,           "encoding-BIG5",           big5_codec);
//     encoding_rs_codec!(encoding_rs::EUC_JP,         euc_jp_decoder,         "encoding-EUC_JP",         euc_jp_codec);
//     encoding_rs_codec!(encoding_rs::EUC_KR,         euc_kr_decoder,         "encoding-EUC_KR",         euc_kr_codec);
//     encoding_rs_codec!(encoding_rs::GB18030,        gb18030_decoder,        "encoding-GB18030",        gb18030_codec);
//     encoding_rs_codec!(encoding_rs::GBK,            gbk_decoder,            "encoding-GBK",            gbk_codec);
//     encoding_rs_codec!(encoding_rs::IBM866,         ibm866_decoder,         "encoding-IBM866",         ibm866_codec);
//     encoding_rs_codec!(encoding_rs::ISO_8859_2,     iso_8859_2_decoder,     "encoding-ISO_8859_2",     iso_8859_2_codec);
//     encoding_rs_codec!(encoding_rs::ISO_8859_3,     iso_8859_3_decoder,     "encoding-ISO_8859_3",     iso_8859_3_codec);
//     encoding_rs_codec!(encoding_rs::ISO_8859_4,     iso_8859_4_decoder,     "encoding-ISO_8859_4",     iso_8859_4_codec);
//     encoding_rs_codec!(encoding_rs::ISO_8859_5,     iso_8859_5_decoder,     "encoding-ISO_8859_5",     iso_8859_5_codec);
//     encoding_rs_codec!(encoding_rs::ISO_8859_6,     iso_8859_6_decoder,     "encoding-ISO_8859_6",     iso_8859_6_codec);
//     encoding_rs_codec!(encoding_rs::ISO_8859_7,     iso_8859_7_decoder,     "encoding-ISO_8859_7",     iso_8859_7_codec);
//     encoding_rs_codec!(encoding_rs::ISO_8859_8,     iso_8859_8_decoder,     "encoding-ISO_8859_8",     iso_8859_8_codec);
//     encoding_rs_codec!(encoding_rs::ISO_8859_8_I,   iso_8859_8_i_decoder,   "encoding-ISO_8859_8_I",   iso_8859_8_i_codec);
//     encoding_rs_codec!(encoding_rs::ISO_8859_10,    iso_8859_10_decoder,    "encoding-ISO_8859_10",    iso_8859_10_codec);
//     encoding_rs_codec!(encoding_rs::ISO_8859_13,    iso_8859_13_decoder,    "encoding-ISO_8859_13",    iso_8859_13_codec);
//     encoding_rs_codec!(encoding_rs::ISO_8859_14,    iso_8859_14_decoder,    "encoding-ISO_8859_14",    iso_8859_14_codec);
//     encoding_rs_codec!(encoding_rs::ISO_8859_15,    iso_8859_15_decoder,    "encoding-ISO_8859_15",    iso_8859_15_codec);
//     encoding_rs_codec!(encoding_rs::ISO_8859_16,    iso_8859_16_decoder,    "encoding-ISO_8859_16",    iso_8859_16_codec);
//     encoding_rs_codec!(encoding_rs::KOI8_R,         koi8_r_decoder,         "encoding-KOI8_R",         koi8_r_codec);
//     encoding_rs_codec!(encoding_rs::KOI8_U,         koi8_u_decoder,         "encoding-KOI8_U",         koi8_u_codec);
//     encoding_rs_codec!(encoding_rs::MACINTOSH,      macintosh_decoder,      "encoding-MACINTOSH",      macintosh_codec);
//     encoding_rs_codec!(encoding_rs::SHIFT_JIS,      shift_jis_decoder,      "encoding-SHIFT_JIS",      shift_jis_codec);
//     encoding_rs_codec!(encoding_rs::WINDOWS_874,    windows_874_decoder,    "encoding-WINDOWS_874",    windows_874_codec);
//     encoding_rs_codec!(encoding_rs::WINDOWS_1250,   windows_1250_decoder,   "encoding-WINDOWS_1250",   windows_1250_codec);
//     encoding_rs_codec!(encoding_rs::WINDOWS_1251,   windows_1251_decoder,   "encoding-WINDOWS_1251",   windows_1251_codec);
//     encoding_rs_codec!(encoding_rs::WINDOWS_1252,   windows_1252_decoder,   "encoding-WINDOWS_1252",   windows_1252_codec);
//     encoding_rs_codec!(encoding_rs::WINDOWS_1253,   windows_1253_decoder,   "encoding-WINDOWS_1253",   windows_1253_codec);
//     encoding_rs_codec!(encoding_rs::WINDOWS_1254,   windows_1254_decoder,   "encoding-WINDOWS_1254",   windows_1254_codec);
//     encoding_rs_codec!(encoding_rs::WINDOWS_1255,   windows_1255_decoder,   "encoding-WINDOWS_1255",   windows_1255_codec);
//     encoding_rs_codec!(encoding_rs::WINDOWS_1256,   windows_1256_decoder,   "encoding-WINDOWS_1256",   windows_1256_codec);
//     encoding_rs_codec!(encoding_rs::WINDOWS_1257,   windows_1257_decoder,   "encoding-WINDOWS_1257",   windows_1257_codec);
//     encoding_rs_codec!(encoding_rs::WINDOWS_1258,   windows_1258_decoder,   "encoding-WINDOWS_1258",   windows_1258_codec);
//     encoding_rs_codec!(encoding_rs::X_MAC_CYRILLIC, x_mac_cyrillic_decoder, "encoding-X_MAC_CYRILLIC", x_mac_cyrillic_codec);
//     encoding_rs_codec!(encoding_rs::X_USER_DEFINED, x_user_defined_decoder, "encoding-X_USER_DEFINED", x_user_defined_codec);
// }

// use encoding_rs_codecs::*;

/// Every codec
/// NOTE: matches for these are returned in the order they are defined here so less likely / weirder
/// codecs should be put further down
pub const ALL_CODEC_GENERATORS: [CodecGenerator; 2] = [
    // 52] = [
    identity_codec,
    // base64 codecs
    base64_codecs,
    // base64_urlsafe_codec,
    // base64_bcrypt_codec,
    // base64_binhex_codec,
    // base64_imap_mutf7_codec,
    /*
    // base32 codecs
    base32_crockford_codec,
    base32_rfc468_codec,
    base32_rfc468_lower_codec,
    base32_rfc468_hex_codec,
    base32_rfc468_hex_lower_codec,
    base32_z_codec,
    // stdlib codecs
    utf16_le_codec,
    utf16_be_codec,
    utf32_le_codec,
    utf32_be_codec,
    // funny ones
    base10_ascii_codec,
    // encoding_rs codecs
    big5_codec,
    euc_jp_codec,
    euc_kr_codec,
    gb18030_codec,
    gbk_codec,
    ibm866_codec,
    iso_8859_2_codec,
    iso_8859_3_codec,
    iso_8859_4_codec,
    iso_8859_5_codec,
    iso_8859_6_codec,
    iso_8859_7_codec,
    iso_8859_8_codec,
    iso_8859_8_i_codec,
    iso_8859_10_codec,
    iso_8859_13_codec,
    iso_8859_14_codec,
    iso_8859_15_codec,
    iso_8859_16_codec,
    koi8_r_codec,
    koi8_u_codec,
    macintosh_codec,
    shift_jis_codec,
    windows_874_codec,
    windows_1250_codec,
    windows_1251_codec,
    windows_1252_codec,
    windows_1253_codec,
    windows_1254_codec,
    windows_1255_codec,
    windows_1256_codec,
    windows_1257_codec,
    windows_1258_codec,
    x_mac_cyrillic_codec,
    x_user_defined_codec,
    */
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_codecs_roundtrip_to_prefix() {
        let mut random_data = Vec::with_capacity(999);
        random_data.resize(random_data.capacity(), 0);
        for b in random_data.iter_mut() {
            *b = rand::random_range(0..=127);
        }
        // all data is valid ascii so this is fine
        let random_str = unsafe { std::str::from_utf8_unchecked(&random_data) };

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
