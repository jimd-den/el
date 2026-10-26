// Tests for `sha256`; sibling of the module they exercise (§4.4).
//
// The vectors are NIST FIPS 180-4 / the CSRC example set, transcribed by hand: a
// digest test whose expected value was recorded from the implementation proves
// only that the implementation is stable, which is not the claim being made.

use super::*;

#[test]
fn the_empty_string_hashes_to_the_nist_value() {
    assert_eq!(
        hex(&digest(b"")),
        "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
    );
}

#[test]
fn abc_hashes_to_the_nist_value() {
    assert_eq!(
        hex(&digest(b"abc")),
        "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
    );
}

#[test]
fn the_two_block_nist_vector_hashes_correctly() {
    let input = b"abcdbcdecdefdefgefghfghighijhijkijkljklmklmnlmnomnopnopq";

    assert_eq!(
        hex(&digest(input)),
        "248d6a61d20638b8e5c026930c3e6039a33ce45964ff2167f6ecedd419db06c1"
    );
}

#[test]
fn a_million_a_characters_hash_correctly() {
    let input = vec![b'a'; 1_000_000];

    assert_eq!(
        hex(&digest(&input)),
        "cdc76e5c9914fb9281a1c7e284d73e67f1809a48a497200e046d39ccc7112cd0"
    );
}

#[test]
fn a_digest_is_thirty_two_bytes_and_sixty_four_hex_digits() {
    let raw = digest(b"el");

    assert_eq!(raw.len(), 32);
    assert_eq!(hex(&raw).len(), 64);
}

#[test]
fn hex_and_unhex_round_trip() {
    let raw = digest(b"round trip");

    assert_eq!(unhex(&hex(&raw)), Some(raw));
}

#[test]
fn unhex_refuses_anything_that_is_not_a_digest() {
    let uppercase = hex(&digest(b"a")).to_uppercase();

    assert_eq!(
        unhex(&uppercase),
        None,
        "two spellings of one object is a bug"
    );
    assert_eq!(unhex("abc"), None);
    assert_eq!(unhex(&"g".repeat(64)), None);
}

#[test]
fn one_changed_byte_changes_the_digest() {
    assert_ne!(digest(b"apply"), digest(b"apply\n"));
}
