use super::*;

fn base10_ascii_decoder(buf: Encoded, _meta: DecoderMetadata) -> MaybeDecoded {
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

fn hex_codecs(data: &str) -> Vec<Codec> {
    let lower_hex = hex::encode(data);
    let upper_hex = hex::encode(data);

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
    ]
}

pub(super) fn ctf_codecs(data: &str) -> Vec<Codec> {
    let mut codecs = vec![base10_ascii_codec(data)];
    codecs.extend(xor_codecs(data));
    codecs.extend(hex_codecs(data));
    codecs
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_make_even() {
        assert_eq!(round_down_to_even(0), 0);
        assert_eq!(round_down_to_even(1), 0);
        assert_eq!(round_down_to_even(2), 2);
        assert_eq!(round_down_to_even(3), 2);
        assert_eq!(round_down_to_even(4), 4);
        assert_eq!(round_down_to_even(usize::MAX), usize::MAX - 1);
    }
}
