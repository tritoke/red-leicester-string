use super::*;

const ENCODINGS: [(&'static encoding_rs::Encoding, &'static str); 35] = [
    (encoding_rs::BIG5, "encoding-BIG5"),
    (encoding_rs::EUC_JP, "encoding-EUC_JP"),
    (encoding_rs::EUC_KR, "encoding-EUC_KR"),
    (encoding_rs::GB18030, "encoding-GB18030"),
    (encoding_rs::GBK, "encoding-GBK"),
    (encoding_rs::IBM866, "encoding-IBM866"),
    (encoding_rs::ISO_8859_2, "encoding-ISO_8859_2"),
    (encoding_rs::ISO_8859_3, "encoding-ISO_8859_3"),
    (encoding_rs::ISO_8859_4, "encoding-ISO_8859_4"),
    (encoding_rs::ISO_8859_5, "encoding-ISO_8859_5"),
    (encoding_rs::ISO_8859_6, "encoding-ISO_8859_6"),
    (encoding_rs::ISO_8859_7, "encoding-ISO_8859_7"),
    (encoding_rs::ISO_8859_8, "encoding-ISO_8859_8"),
    (encoding_rs::ISO_8859_8_I, "encoding-ISO_8859_8_I"),
    (encoding_rs::ISO_8859_10, "encoding-ISO_8859_10"),
    (encoding_rs::ISO_8859_13, "encoding-ISO_8859_13"),
    (encoding_rs::ISO_8859_14, "encoding-ISO_8859_14"),
    (encoding_rs::ISO_8859_15, "encoding-ISO_8859_15"),
    (encoding_rs::ISO_8859_16, "encoding-ISO_8859_16"),
    (encoding_rs::KOI8_R, "encoding-KOI8_R"),
    (encoding_rs::KOI8_U, "encoding-KOI8_U"),
    (encoding_rs::MACINTOSH, "encoding-MACINTOSH"),
    (encoding_rs::SHIFT_JIS, "encoding-SHIFT_JIS"),
    (encoding_rs::WINDOWS_874, "encoding-WINDOWS_874"),
    (encoding_rs::WINDOWS_1250, "encoding-WINDOWS_1250"),
    (encoding_rs::WINDOWS_1251, "encoding-WINDOWS_1251"),
    (encoding_rs::WINDOWS_1252, "encoding-WINDOWS_1252"),
    (encoding_rs::WINDOWS_1253, "encoding-WINDOWS_1253"),
    (encoding_rs::WINDOWS_1254, "encoding-WINDOWS_1254"),
    (encoding_rs::WINDOWS_1255, "encoding-WINDOWS_1255"),
    (encoding_rs::WINDOWS_1256, "encoding-WINDOWS_1256"),
    (encoding_rs::WINDOWS_1257, "encoding-WINDOWS_1257"),
    (encoding_rs::WINDOWS_1258, "encoding-WINDOWS_1258"),
    (encoding_rs::X_MAC_CYRILLIC, "encoding-X_MAC_CYRILLIC"),
    (encoding_rs::X_USER_DEFINED, "encoding-X_USER_DEFINED"),
];

fn encoding_rs_decoder(buf: Encoded, meta: DecoderMetadata) -> Decoded {
    let encoding: &encoding_rs::Encoding = retrieve_metadata(meta);
    let (decoded, _, _) = encoding.decode(buf.as_ref());
    // Cow -> String -> Vec<[u8]> -> [u8]
    Some(decoded.into_owned().into_bytes().into())
}

pub(super) fn encoding_rs_codecs(data: &str) -> Vec<Codec> {
    let mut codecs = Vec::with_capacity(ENCODINGS.len());

    for (encoding, codec_name) in ENCODINGS {
        let (encoded, _, _) = encoding.encode(data);
        codecs.push(Codec {
            encoded: encoded.into_owned().into(),
            name: codec_name,
            decoder: encoding_rs_decoder,
            metadata: Some(encoding as &dyn ThreadSafeAny),
        });
    }

    codecs
}
