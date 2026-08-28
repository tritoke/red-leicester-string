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
    let engine: &base64::engine::GeneralPurpose = retrieve_metadata(meta);

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
            metadata: Some(engine as &dyn ThreadSafeAny),
        });
    }

    codecs
}
