use super::*;

fn utf8_decoder(buf: Encoded, _meta: DecoderMetadata) -> MaybeDecoded {
    Some(buf)
}

fn utf8_codec(data: &str) -> Codec {
    Codec {
        encoded: Box::from(data.as_bytes()),
        name: "UTF8",
        decoder: utf8_decoder,
        metadata: None,
    }
}

fn utf16_le_decoder(buf: Encoded, _meta: DecoderMetadata) -> MaybeDecoded {
    let decoded = String::from_utf16le_lossy(&buf);
    Some(decoded.into_bytes().into())
}

fn utf16_le_codec(data: &str) -> Codec {
    let utf16_codepoints = data.encode_utf16();
    let utf16_bytes: Vec<u8> = utf16_codepoints
        .flat_map(|codepoint| codepoint.to_le_bytes())
        .collect();

    Codec {
        encoded: utf16_bytes.into(),
        name: "UTF-16-LE",
        decoder: utf16_le_decoder,
        metadata: None,
    }
}

fn utf16_be_decoder(buf: Encoded, _meta: DecoderMetadata) -> MaybeDecoded {
    let decoded = String::from_utf16be_lossy(&buf);
    Some(decoded.into_bytes().into())
}

fn utf16_be_codec(data: &str) -> Codec {
    let utf16_codepoints = data.encode_utf16();
    let utf16_bytes: Vec<u8> = utf16_codepoints
        .flat_map(|codepoint| codepoint.to_be_bytes())
        .collect();

    Codec {
        encoded: utf16_bytes.into(),
        name: "UTF-16-BE",
        decoder: utf16_be_decoder,
        metadata: None,
    }
}

fn utf32_le_decoder(buf: Encoded, _meta: DecoderMetadata) -> MaybeDecoded {
    let (utf32_byte_pairs, _trailing): (&[[u8; 4]], &[u8]) = buf.as_chunks();
    let decoded: String = utf32_byte_pairs
        .iter()
        .map(|codepoint| {
            char::from_u32(u32::from_le_bytes(*codepoint)).unwrap_or(char::REPLACEMENT_CHARACTER)
        })
        .collect();

    Some(decoded.into_bytes().into())
}

fn utf32_le_codec(data: &str) -> Codec {
    let utf32_codepoints = data.chars().map(|c| c as u32);
    let utf32_bytes: Vec<u8> = utf32_codepoints
        .flat_map(|codepoint| codepoint.to_le_bytes())
        .collect();

    Codec {
        encoded: utf32_bytes.into(),
        name: "UTF-32-LE",
        decoder: utf32_le_decoder,
        metadata: None,
    }
}

fn utf32_be_decoder(buf: Encoded, _meta: DecoderMetadata) -> MaybeDecoded {
    let (utf32_byte_pairs, _trailing): (&[[u8; 4]], &[u8]) = buf.as_chunks();
    let decoded: String = utf32_byte_pairs
        .iter()
        .map(|codepoint| {
            char::from_u32(u32::from_be_bytes(*codepoint)).unwrap_or(char::REPLACEMENT_CHARACTER)
        })
        .collect();

    Some(decoded.into_bytes().into())
}

fn utf32_be_codec(data: &str) -> Codec {
    let utf32_codepoints = data.chars().map(|c| c as u32);
    let utf32_bytes: Vec<u8> = utf32_codepoints
        .flat_map(|codepoint| codepoint.to_be_bytes())
        .collect();

    Codec {
        encoded: utf32_bytes.into(),
        name: "UTF-32-BE",
        decoder: utf32_be_decoder,
        metadata: None,
    }
}

pub(super) fn common_encodings(data: &str) -> Vec<Codec> {
    vec![
        utf8_codec(data),
        utf16_le_codec(data),
        utf16_be_codec(data),
        utf32_le_codec(data),
        utf32_be_codec(data),
    ]
}
