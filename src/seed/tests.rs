use std::collections::HashSet;

use super::*;

/// The first three numbers of a stream.
fn first_three(mut rng: Rng) -> [u64; 3] {
    [rng.next_u64(), rng.next_u64(), rng.next_u64()]
}

#[test]
fn a_source_is_deterministic_and_gives_a_thousand_different_seeds() {
    let mut one = SeedSource::new(42);
    let mut two = SeedSource::new(42);
    let seeds: Vec<u64> = (0..1000).map(|_| one.next()).collect();
    assert!(seeds.iter().all(|seed| *seed == two.next()));
    assert_eq!(seeds.iter().collect::<HashSet<_>>().len(), 1000);
    assert_ne!(
        SeedSource::new(43).next(),
        seeds[0],
        "another start, another run"
    );
}

#[test]
fn a_source_walks_the_splitmix64_sequence_and_the_default_start_is_fixed() {
    // The well-known SplitMix64 outputs for a start of 0, so a change to the walk fails here.
    let mut source = SeedSource::new(0);
    assert_eq!(source.next(), 0xE220_A839_7B1D_CDAF);
    assert_eq!(source.next(), 0x6E78_9E6A_A1B9_65F4);
    assert_eq!(source.next(), 0x06C4_5D18_8009_454F);

    let mut default = SeedSource::default();
    assert_eq!(default.next(), 0x9896_D4F2_C9EF_05A3);
    assert_eq!(default.next(), 0xD9E0_65AB_E0EE_70F0);
    assert_eq!(SeedSource::default(), SeedSource::default());
}

#[test]
fn three_streams_start_with_these_exact_numbers() {
    // Computed apart from this code (FNV-1a of the label, then three SplitMix steps), so a change
    // to the mixing fails loudly: it would also change every run's enemies.
    assert_eq!(
        first_three(stream(1, "enemies", 0)),
        [
            0x646E_A847_B095_D548,
            0x1075_F690_9468_FFBE,
            0x22D1_CDD5_3050_FFE5
        ]
    );
    assert_eq!(
        first_three(stream(0xDEAD_BEEF, "enemies", 7)),
        [
            0xA706_4B29_FE8A_7073,
            0x01F4_0D74_A9EB_FB75,
            0x2B8B_3507_6929_F2C9
        ]
    );
    assert_eq!(
        first_three(stream(u64::MAX, "loot", u64::MAX)),
        [
            0xB826_EAC0_7C9C_0584,
            0x7629_3A3F_938F_F748,
            0xFA63_01EB_D33E_6B2E
        ]
    );
}

#[test]
fn the_same_inputs_give_the_same_stream() {
    assert_eq!(stream(9, "enemies", 3), stream(9, "enemies", 3));
    assert_eq!(
        first_three(stream(9, "enemies", 3)),
        first_three(stream(9, "enemies", 3))
    );
}

#[test]
fn streams_with_different_labels_keys_or_seeds_differ() {
    let base = first_three(stream(5, "enemies", 1));
    assert_ne!(base, first_three(stream(5, "loot", 1)), "label");
    assert_ne!(base, first_three(stream(5, "enemies", 2)), "key");
    assert_ne!(base, first_three(stream(6, "enemies", 1)), "seed");
    // A label is not just its length or its first letter.
    assert_ne!(
        first_three(stream(5, "ab", 1)),
        first_three(stream(5, "ba", 1))
    );
    // A thousand keys under one seed and label never share a starting point.
    let starts: HashSet<u64> = (0..1000)
        .map(|key| stream(5, "enemies", key).next_u64())
        .collect();
    assert_eq!(starts.len(), 1000);
}

#[test]
fn a_seed_is_sixteen_lowercase_hex_digits() {
    assert_eq!(format_seed(0), "0000000000000000");
    assert_eq!(format_seed(0xABC), "0000000000000abc");
    assert_eq!(format_seed(u64::MAX), "ffffffffffffffff");
    assert_eq!(format_seed(0xDEAD_BEEF_0000_0001), "deadbeef00000001");
}

#[test]
fn format_and_parse_round_trip() {
    for seed in [0, 1, 0xABC, 0xDEAD_BEEF, u64::MAX, 0x0123_4567_89AB_CDEF] {
        assert_eq!(parse_seed(&format_seed(seed)), Ok(seed));
    }
    let mut source = SeedSource::new(7);
    for _ in 0..100 {
        let seed = source.next();
        assert_eq!(parse_seed(&format_seed(seed)), Ok(seed));
    }
}

#[test]
fn one_to_sixteen_digits_in_any_case_with_an_optional_prefix_are_accepted() {
    assert_eq!(parse_seed("0"), Ok(0));
    assert_eq!(parse_seed("f"), Ok(15));
    assert_eq!(parse_seed("ABC"), Ok(0xABC));
    assert_eq!(parse_seed("aBc"), Ok(0xABC));
    assert_eq!(parse_seed("0xabc"), Ok(0xABC));
    assert_eq!(parse_seed("0XABC"), Ok(0xABC));
    assert_eq!(parse_seed("ffffffffffffffff"), Ok(u64::MAX));
    assert_eq!(parse_seed("0xFFFFFFFFFFFFFFFF"), Ok(u64::MAX));
    assert_eq!(
        parse_seed("0000000000000001"),
        Ok(1),
        "16 digits, leading zeros"
    );
}

#[test]
fn bad_input_is_refused_with_the_reason() {
    let empty = parse_seed("").unwrap_err();
    assert!(empty.contains("empty"), "{empty}");

    let long = parse_seed("12345678901234567").unwrap_err();
    assert!(long.contains("at most 16") && long.contains("17"), "{long}");
    let long_with_prefix = parse_seed("0x12345678901234567").unwrap_err();
    assert!(long_with_prefix.contains("17"), "{long_with_prefix}");

    let letter = parse_seed("12g4").unwrap_err();
    assert!(
        letter.contains("'g'") && letter.contains("hex digit"),
        "{letter}"
    );

    let bare_prefix = parse_seed("0x").unwrap_err();
    assert!(bare_prefix.contains("after 0x"), "{bare_prefix}");
}

#[test]
fn spaces_signs_and_other_forms_are_refused() {
    for text in [
        " ", " 12", "12 ", "1 2", "+12", "-1", "0x 1", "0x0x1", "x12", "1_000", "１２",
    ] {
        assert!(parse_seed(text).is_err(), "{text:?} must be refused");
    }
    assert!(parse_seed("12 ").unwrap_err().contains("' '"));
}
