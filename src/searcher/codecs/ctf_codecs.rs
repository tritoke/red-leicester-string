use super::*;

fn base10_ascii_decoder(buf: Encoded, _meta: DecoderMetadata) -> Decoded {
    // this is a strict upper bound from the length of buf
    let mut acc = String::with_capacity(buf.len() / 2);

    let mut cur = 0;
    for c in buf {
        if !c.is_ascii_digit() {
            break;
        }

        dbg!(c, cur);
        cur *= 10;
        cur += c - b'0';

        if cur >= b' ' {
            acc.push(cur as char);
            cur = 0;
        }
    }

    Some(acc.into_bytes().into())
}

fn base10_ascii_codec(data: &str) -> Codec {
    use std::io::Write;

    let mut buf = Vec::with_capacity(data.chars().count() * 3);
    for c in data.chars() {
        write!(&mut buf, "{}", c as u32).unwrap();
    }

    Codec {
        encoded: buf.into(),
        name: "base10-ascii",
        decoder: base10_ascii_decoder,
        metadata: None,
    }
}

pub(super) fn ctf_codecs(data: &str) -> Vec<Codec> {
    vec![base10_ascii_codec(data)]
}
