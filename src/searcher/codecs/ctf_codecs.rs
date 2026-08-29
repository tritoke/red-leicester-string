use std::sync::LazyLock;

use super::*;

fn base10_ascii_decoder(buf: Encoded, _meta: DecoderMetadata) -> MaybeDecoded {
    // this is a strict upper bound from the length of buf
    let mut acc = String::with_capacity(buf.len() / 2);

    let mut cur = 0;
    for c in buf {
        if !c.is_ascii_digit() {
            break;
        }

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

// Either I allocate these statically or I Box::leak them so like 🤷
#[rustfmt::skip]
const XOR_CONSTANTS: &'static [(u8, DecoderName)] = &[
    (1,   "XOR_1"),   (2,   "XOR_2"),   (3,   "XOR_3"),   (4,   "XOR_4"),   (5,   "XOR_5"),   (6,   "XOR_6"),   (7,   "XOR_7"),     (8, "XOR_8"),
    (9,   "XOR_9"),   (10,  "XOR_10"),  (11,  "XOR_11"),  (12,  "XOR_12"),  (13,  "XOR_13"),  (14,  "XOR_14"),  (15,  "XOR_15"),   (16, "XOR_16"),
    (17,  "XOR_17"),  (18,  "XOR_18"),  (19,  "XOR_19"),  (20,  "XOR_20"),  (21,  "XOR_21"),  (22,  "XOR_22"),  (23,  "XOR_23"),   (24, "XOR_24"),
    (25,  "XOR_25"),  (26,  "XOR_26"),  (27,  "XOR_27"),  (28,  "XOR_28"),  (29,  "XOR_29"),  (30,  "XOR_30"),  (31,  "XOR_31"),   (32, "XOR_32"),
    (33,  "XOR_33"),  (34,  "XOR_34"),  (35,  "XOR_35"),  (36,  "XOR_36"),  (37,  "XOR_37"),  (38,  "XOR_38"),  (39,  "XOR_39"),   (40, "XOR_40"),
    (41,  "XOR_41"),  (42,  "XOR_42"),  (43,  "XOR_43"),  (44,  "XOR_44"),  (45,  "XOR_45"),  (46,  "XOR_46"),  (47,  "XOR_47"),   (48, "XOR_48"),
    (49,  "XOR_49"),  (50,  "XOR_50"),  (51,  "XOR_51"),  (52,  "XOR_52"),  (53,  "XOR_53"),  (54,  "XOR_54"),  (55,  "XOR_55"),   (56, "XOR_56"),
    (57,  "XOR_57"),  (58,  "XOR_58"),  (59,  "XOR_59"),  (60,  "XOR_60"),  (61,  "XOR_61"),  (62,  "XOR_62"),  (63,  "XOR_63"),   (64, "XOR_64"),
    (65,  "XOR_65"),  (66,  "XOR_66"),  (67,  "XOR_67"),  (68,  "XOR_68"),  (69,  "XOR_69"),  (70,  "XOR_70"),  (71,  "XOR_71"),   (72, "XOR_72"),
    (73,  "XOR_73"),  (74,  "XOR_74"),  (75,  "XOR_75"),  (76,  "XOR_76"),  (77,  "XOR_77"),  (78,  "XOR_78"),  (79,  "XOR_79"),   (80, "XOR_80"),
    (81,  "XOR_81"),  (82,  "XOR_82"),  (83,  "XOR_83"),  (84,  "XOR_84"),  (85,  "XOR_85"),  (86,  "XOR_86"),  (87,  "XOR_87"),   (88, "XOR_88"),
    (89,  "XOR_89"),  (90,  "XOR_90"),  (91,  "XOR_91"),  (92,  "XOR_92"),  (93,  "XOR_93"),  (94,  "XOR_94"),  (95,  "XOR_95"),   (96, "XOR_96"),
    (97,  "XOR_97"),  (98,  "XOR_98"),  (99,  "XOR_99"),  (100, "XOR_100"), (101, "XOR_101"), (102, "XOR_102"), (103, "XOR_103"), (104, "XOR_104"),
    (105, "XOR_105"), (106, "XOR_106"), (107, "XOR_107"), (108, "XOR_108"), (109, "XOR_109"), (110, "XOR_110"), (111, "XOR_111"), (112, "XOR_112"),
    (113, "XOR_113"), (114, "XOR_114"), (115, "XOR_115"), (116, "XOR_116"), (117, "XOR_117"), (118, "XOR_118"), (119, "XOR_119"), (120, "XOR_120"),
    (121, "XOR_121"), (122, "XOR_122"), (123, "XOR_123"), (124, "XOR_124"), (125, "XOR_125"), (126, "XOR_126"), (127, "XOR_127"), (128, "XOR_128"),
    (129, "XOR_129"), (130, "XOR_130"), (131, "XOR_131"), (132, "XOR_132"), (133, "XOR_133"), (134, "XOR_134"), (135, "XOR_135"), (136, "XOR_136"),
    (137, "XOR_137"), (138, "XOR_138"), (139, "XOR_139"), (140, "XOR_140"), (141, "XOR_141"), (142, "XOR_142"), (143, "XOR_143"), (144, "XOR_144"),
    (145, "XOR_145"), (146, "XOR_146"), (147, "XOR_147"), (148, "XOR_148"), (149, "XOR_149"), (150, "XOR_150"), (151, "XOR_151"), (152, "XOR_152"),
    (153, "XOR_153"), (154, "XOR_154"), (155, "XOR_155"), (156, "XOR_156"), (157, "XOR_157"), (158, "XOR_158"), (159, "XOR_159"), (160, "XOR_160"),
    (161, "XOR_161"), (162, "XOR_162"), (163, "XOR_163"), (164, "XOR_164"), (165, "XOR_165"), (166, "XOR_166"), (167, "XOR_167"), (168, "XOR_168"),
    (169, "XOR_169"), (170, "XOR_170"), (171, "XOR_171"), (172, "XOR_172"), (173, "XOR_173"), (174, "XOR_174"), (175, "XOR_175"), (176, "XOR_176"),
    (177, "XOR_177"), (178, "XOR_178"), (179, "XOR_179"), (180, "XOR_180"), (181, "XOR_181"), (182, "XOR_182"), (183, "XOR_183"), (184, "XOR_184"),
    (185, "XOR_185"), (186, "XOR_186"), (187, "XOR_187"), (188, "XOR_188"), (189, "XOR_189"), (190, "XOR_190"), (191, "XOR_191"), (192, "XOR_192"),
    (193, "XOR_193"), (194, "XOR_194"), (195, "XOR_195"), (196, "XOR_196"), (197, "XOR_197"), (198, "XOR_198"), (199, "XOR_199"), (200, "XOR_200"),
    (201, "XOR_201"), (202, "XOR_202"), (203, "XOR_203"), (204, "XOR_204"), (205, "XOR_205"), (206, "XOR_206"), (207, "XOR_207"), (208, "XOR_208"),
    (209, "XOR_209"), (210, "XOR_210"), (211, "XOR_211"), (212, "XOR_212"), (213, "XOR_213"), (214, "XOR_214"), (215, "XOR_215"), (216, "XOR_216"),
    (217, "XOR_217"), (218, "XOR_218"), (219, "XOR_219"), (220, "XOR_220"), (221, "XOR_221"), (222, "XOR_222"), (223, "XOR_223"), (224, "XOR_224"),
    (225, "XOR_225"), (226, "XOR_226"), (227, "XOR_227"), (228, "XOR_228"), (229, "XOR_229"), (230, "XOR_230"), (231, "XOR_231"), (232, "XOR_232"),
    (233, "XOR_233"), (234, "XOR_234"), (235, "XOR_235"), (236, "XOR_236"), (237, "XOR_237"), (238, "XOR_238"), (239, "XOR_239"), (240, "XOR_240"),
    (241, "XOR_241"), (242, "XOR_242"), (243, "XOR_243"), (244, "XOR_244"), (245, "XOR_245"), (246, "XOR_246"), (247, "XOR_247"), (248, "XOR_248"),
    (249, "XOR_249"), (250, "XOR_250"), (251, "XOR_251"), (252, "XOR_252"), (253, "XOR_253"), (254, "XOR_254"), (255, "XOR_255"),
];

fn xor_decoder(mut buf: Encoded, meta: DecoderMetadata) -> MaybeDecoded {
    let xor_constant: &'static u8 = retrieve_metadata(meta);
    for b in &mut buf {
        *b ^= *xor_constant;
    }

    Some(buf)
}

fn xor_codecs(data: &str) -> impl Iterator<Item = Codec> {
    XOR_CONSTANTS.into_iter().map(|(xor_constant, codec_name)| {
        let mut bytes = data.as_bytes().to_owned();
        for b in bytes.iter_mut() {
            *b ^= *xor_constant;
        }

        Codec {
            encoded: bytes.into(),
            name: codec_name,
            decoder: xor_decoder,
            metadata: Some(xor_constant as &dyn ThreadSafeAny),
        }
    })
}

#[inline(always)]
fn round_down_to_even(i: usize) -> usize {
    i & (!1)
}

fn hex_decoder(buf: Encoded, _meta: DecoderMetadata) -> MaybeDecoded {
    for (i, c) in buf.iter().enumerate() {
        if !c.is_ascii_hexdigit() {
            let decoded = hex::decode(&buf[..round_down_to_even(i)])
                .expect("aparently is_ascii_hexdigit is wrong?");
            return Some(decoded.into());
        }
    }

    let decoded = hex::decode(&buf[..round_down_to_even(buf.len())])
        .expect("aparently is_ascii_hexdigit is wrong?");
    Some(decoded.into())
}

fn raw_hex_decoder(mut buf: Encoded, _meta: DecoderMetadata) -> MaybeDecoded {
    // mutate the buffer into a valid hex string and decode that
    for b in &mut buf {
        match *b {
            0..0xA => {
                *b += b'0';
            }
            0xA..=0xF => {
                *b += b'a' - 0xA;
            }
            _ => {
                break;
            }
        }
    }

    hex_decoder(buf, None)
}

fn hex_codecs(data: &str) -> Vec<Codec> {
    let lower_hex = hex::encode(data);
    let upper_hex = hex::encode(data);
    let raw_hex: Vec<u8> = lower_hex
        .bytes()
        .map(|b| match b {
            b'0'..=b'9' => b - b'0',
            b'a'..=b'f' => b - b'a' + 0xA,
            _ => unreachable!("hex encoded data should not contain any other character"),
        })
        .collect();

    vec![
        Codec {
            encoded: lower_hex.into_bytes().into(),
            name: "hex-lower",
            decoder: hex_decoder,
            metadata: None,
        },
        Codec {
            encoded: upper_hex.into_bytes().into(),
            name: "hex-upper",
            decoder: hex_decoder,
            metadata: None,
        },
        Codec {
            encoded: raw_hex.into(),
            name: "raw-hex",
            decoder: raw_hex_decoder,
            metadata: None,
        },
    ]
}

fn decode_binary_block(block: &[u8; 8]) -> Option<u8> {
    let mut byte = 0;
    for b in block {
        if *b != b'0' && *b != b'1' {
            return None;
        }

        byte = (byte << 1) | (*b - b'0');
    }

    Some(byte)
}

fn binary_decoder(buf: Encoded, _meta: DecoderMetadata) -> MaybeDecoded {
    binary_decoder_inner(&buf)
}

fn binary_decoder_inner(buf: &[u8]) -> MaybeDecoded {
    let (buf_blocks, _trailing) = buf.as_chunks::<8>();

    let decoded: Vec<u8> = buf_blocks
        .iter()
        .map(decode_binary_block)
        .take_while(Option::is_some)
        // SAFETY: we just checked it was Some() 👍
        .map(|b| unsafe { b.unwrap_unchecked() })
        .collect();

    Some(decoded.into())
}

fn bytewise_binary_decoder(mut buf: Encoded, meta: DecoderMetadata) -> MaybeDecoded {
    let &(zero_byte, one_byte): &(u8, u8) = retrieve_metadata(meta);

    for (i, b) in buf.iter_mut().enumerate() {
        if *b == zero_byte {
            *b = b'0';
        } else if *b == one_byte {
            *b = b'1';
        } else {
            return binary_decoder_inner(&buf[..i]);
        }
    }

    return binary_decoder_inner(&buf);
}

// Allocate these on first use as its quite a lot of data on its own lol
static BYTEWISE_DECODER_NAMES: LazyLock<Box<[((u8, u8), String)]>> = LazyLock::new(|| {
    let mut names = Vec::with_capacity(255 * 256);

    // enable gamble when testing in release mode as well as when --gamble is passed
    let gamble_flag =
        crate::GAMBLE.with(|gamble| gamble.load(std::sync::atomic::Ordering::Relaxed));
    let release_mode_testing = cfg!(all(test, not(debug_assertions)));
    let gamble = gamble_flag || release_mode_testing;
    for zero_byte in u8::MIN..=u8::MAX {
        for one_byte in u8::MIN..=u8::MAX {
            if zero_byte == one_byte {
                continue;
            }

            let codec_name = format!("binary-0=0x{zero_byte:02x}-1=0x{one_byte:02x}");
            names.push(((zero_byte, one_byte), codec_name));

            if !gamble {
                break;
            }
        }

        if !gamble {
            break;
        }
    }

    names.into()
});

fn binary_codecs(data: &str) -> Vec<Codec> {
    use std::io::Write;

    let mut binary = Vec::with_capacity(data.as_bytes().len() * 8);
    for byte in data.as_bytes() {
        write!(&mut binary, "{byte:08b}").unwrap();
    }

    let mut codecs = Vec::with_capacity(255 * 256 + 1);
    codecs.push(Codec {
        encoded: binary.clone().into(),
        name: "binary",
        decoder: binary_decoder,
        metadata: None,
    });

    let gamble = crate::GAMBLE.with(|gamble| gamble.load(std::sync::atomic::Ordering::Relaxed));
    for (pair @ (zero_byte, one_byte), decoder_name) in BYTEWISE_DECODER_NAMES.iter() {
        let mut modified_binary = binary.clone();
        for b in &mut modified_binary {
            if *b == b'0' {
                *b = *zero_byte;
            } else {
                *b = *one_byte;
            }
        }

        codecs.push(Codec {
            encoded: modified_binary.into(),
            name: decoder_name.as_str(),
            decoder: bytewise_binary_decoder,
            metadata: Some(pair as &dyn ThreadSafeAny),
        });

        // if we aren't gambling then only load the first raw pattern - zero_byte = 0, one_byte = 1.
        if !gamble {
            break;
        }
    }

    codecs
}

fn rotate_around(c: u8, base: u8, rot: u8, period: u8) -> u8 {
    let off = c - base;
    let new_off = off.wrapping_add(rot) % period;
    base + new_off
}

const ROT13_CODEC_PAIRS: &'static [(u8, DecoderName)] = &[
    (1, "ROT-13-rot=1"),
    (2, "ROT-13-rot=2"),
    (3, "ROT-13-rot=3"),
    (4, "ROT-13-rot=4"),
    (5, "ROT-13-rot=5"),
    (6, "ROT-13-rot=6"),
    (7, "ROT-13-rot=7"),
    (8, "ROT-13-rot=8"),
    (9, "ROT-13-rot=9"),
    (10, "ROT-13-rot=10"),
    (11, "ROT-13-rot=11"),
    (12, "ROT-13-rot=12"),
    (13, "ROT-13-rot=13"),
    (14, "ROT-13-rot=14"),
    (15, "ROT-13-rot=15"),
    (16, "ROT-13-rot=16"),
    (17, "ROT-13-rot=17"),
    (18, "ROT-13-rot=18"),
    (19, "ROT-13-rot=19"),
    (20, "ROT-13-rot=20"),
    (21, "ROT-13-rot=21"),
    (22, "ROT-13-rot=22"),
    (23, "ROT-13-rot=23"),
    (24, "ROT-13-rot=24"),
    (25, "ROT-13-rot=25"),
];

fn rot13_encode(data: &str, rot: u8) -> Box<[u8]> {
    let mut to_encode = data.as_bytes().to_owned();
    for c in &mut to_encode {
        match *c {
            b'a'..=b'z' => {
                *c = rotate_around(*c, b'a', rot, 26);
            }
            b'A'..=b'Z' => {
                *c = rotate_around(*c, b'A', rot, 26);
            }
            _ => (),
        }
    }

    to_encode.into()
}

fn rot13_decode(mut buf: Encoded, meta: DecoderMetadata) -> MaybeDecoded {
    let rot: u8 = *retrieve_metadata(meta);

    for c in &mut buf {
        match *c {
            b'a'..=b'z' => {
                *c = rotate_around(*c, b'a', 26 - rot, 26);
            }
            b'A'..=b'Z' => {
                *c = rotate_around(*c, b'A', 26 - rot, 26);
            }
            _ => (),
        }
    }

    Some(buf)
}

fn rot13_codecs(data: &str) -> impl Iterator<Item = Codec> {
    ROT13_CODEC_PAIRS.iter().map(|&(ref rot, name)| Codec {
        encoded: rot13_encode(data, *rot),
        name,
        decoder: rot13_decode,
        metadata: Some(rot as &dyn ThreadSafeAny),
    })
}

const ROT47_CODEC_PAIRS: &'static [(u8, DecoderName)] = &[
    (1, "ROT-47-rot=1"),
    (2, "ROT-47-rot=2"),
    (3, "ROT-47-rot=3"),
    (4, "ROT-47-rot=4"),
    (5, "ROT-47-rot=5"),
    (6, "ROT-47-rot=6"),
    (7, "ROT-47-rot=7"),
    (8, "ROT-47-rot=8"),
    (9, "ROT-47-rot=9"),
    (10, "ROT-47-rot=10"),
    (11, "ROT-47-rot=11"),
    (12, "ROT-47-rot=12"),
    (13, "ROT-47-rot=13"),
    (14, "ROT-47-rot=14"),
    (15, "ROT-47-rot=15"),
    (16, "ROT-47-rot=16"),
    (17, "ROT-47-rot=17"),
    (18, "ROT-47-rot=18"),
    (19, "ROT-47-rot=19"),
    (20, "ROT-47-rot=20"),
    (21, "ROT-47-rot=21"),
    (22, "ROT-47-rot=22"),
    (23, "ROT-47-rot=23"),
    (24, "ROT-47-rot=24"),
    (25, "ROT-47-rot=25"),
    (26, "ROT-47-rot=26"),
    (27, "ROT-47-rot=27"),
    (28, "ROT-47-rot=28"),
    (29, "ROT-47-rot=29"),
    (30, "ROT-47-rot=30"),
    (31, "ROT-47-rot=31"),
    (32, "ROT-47-rot=32"),
    (33, "ROT-47-rot=33"),
    (34, "ROT-47-rot=34"),
    (35, "ROT-47-rot=35"),
    (36, "ROT-47-rot=36"),
    (37, "ROT-47-rot=37"),
    (38, "ROT-47-rot=38"),
    (39, "ROT-47-rot=39"),
    (40, "ROT-47-rot=40"),
    (41, "ROT-47-rot=41"),
    (42, "ROT-47-rot=42"),
    (43, "ROT-47-rot=43"),
    (44, "ROT-47-rot=44"),
    (45, "ROT-47-rot=45"),
    (46, "ROT-47-rot=46"),
    (47, "ROT-47-rot=47"),
    (48, "ROT-47-rot=48"),
    (49, "ROT-47-rot=49"),
    (50, "ROT-47-rot=50"),
    (51, "ROT-47-rot=51"),
    (52, "ROT-47-rot=52"),
    (53, "ROT-47-rot=53"),
    (54, "ROT-47-rot=54"),
    (55, "ROT-47-rot=55"),
    (56, "ROT-47-rot=56"),
    (57, "ROT-47-rot=57"),
    (58, "ROT-47-rot=58"),
    (59, "ROT-47-rot=59"),
    (60, "ROT-47-rot=60"),
    (61, "ROT-47-rot=61"),
    (62, "ROT-47-rot=62"),
    (63, "ROT-47-rot=63"),
    (64, "ROT-47-rot=64"),
    (65, "ROT-47-rot=65"),
    (66, "ROT-47-rot=66"),
    (67, "ROT-47-rot=67"),
    (68, "ROT-47-rot=68"),
    (69, "ROT-47-rot=69"),
    (70, "ROT-47-rot=70"),
    (71, "ROT-47-rot=71"),
    (72, "ROT-47-rot=72"),
    (73, "ROT-47-rot=73"),
    (74, "ROT-47-rot=74"),
    (75, "ROT-47-rot=75"),
    (76, "ROT-47-rot=76"),
    (77, "ROT-47-rot=77"),
    (78, "ROT-47-rot=78"),
    (79, "ROT-47-rot=79"),
    (80, "ROT-47-rot=80"),
    (81, "ROT-47-rot=81"),
    (82, "ROT-47-rot=82"),
    (83, "ROT-47-rot=83"),
    (84, "ROT-47-rot=84"),
    (85, "ROT-47-rot=85"),
    (86, "ROT-47-rot=86"),
    (87, "ROT-47-rot=87"),
    (88, "ROT-47-rot=88"),
    (89, "ROT-47-rot=89"),
    (90, "ROT-47-rot=90"),
    (91, "ROT-47-rot=91"),
    (92, "ROT-47-rot=92"),
    (93, "ROT-47-rot=93"),
];

fn rot47_encode(data: &str, rot: u8) -> Box<[u8]> {
    let mut to_encode = data.as_bytes().to_owned();
    for c in &mut to_encode {
        if (b'!'..=b'~').contains(c) {
            *c = rotate_around(*c, b'!', rot, 94);
        }
    }

    to_encode.into()
}

fn rot47_decode(mut buf: Encoded, meta: DecoderMetadata) -> MaybeDecoded {
    let rot: u8 = *retrieve_metadata(meta);

    for c in &mut buf {
        if (b'!'..=b'~').contains(c) {
            *c = rotate_around(*c, b'!', 94 - rot, 94);
        }
    }

    Some(buf)
}

fn rot47_codecs(data: &str) -> impl Iterator<Item = Codec> {
    ROT47_CODEC_PAIRS.iter().map(|&(ref rot, name)| Codec {
        encoded: rot47_encode(data, *rot),
        name,
        decoder: rot47_decode,
        metadata: Some(rot as &dyn ThreadSafeAny),
    })
}

pub(super) fn ctf_codecs(data: &str) -> Vec<Codec> {
    let mut codecs = vec![base10_ascii_codec(data)];
    codecs.extend(xor_codecs(data));
    codecs.extend(hex_codecs(data));
    codecs.extend(binary_codecs(data));
    codecs.extend(rot13_codecs(data));
    codecs.extend(rot47_codecs(data));
    codecs
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_round_down_to_even() {
        assert_eq!(round_down_to_even(0), 0);
        assert_eq!(round_down_to_even(1), 0);
        assert_eq!(round_down_to_even(2), 2);
        assert_eq!(round_down_to_even(3), 2);
        assert_eq!(round_down_to_even(4), 4);
        assert_eq!(round_down_to_even(usize::MAX), usize::MAX - 1);
    }

    #[test]
    fn test_xor_constant_table_is_correct() {
        for (i, &(c, name)) in XOR_CONSTANTS.iter().enumerate() {
            assert_eq!(i + 1, c as usize, "Constants are not in order");
            assert_eq!(format!("XOR_{c}"), name, "Name doesn't match constant");
        }
    }
}
