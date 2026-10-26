// Tests for `object`; sibling of the module they exercise (§4.4).

use super::*;

#[test]
fn a_blob_is_addressed_by_its_own_digest() {
    let bytes = b"fn run() {}\n";
    let digest = of(bytes);

    assert_eq!(
        path(&digest),
        format!("objects/{}/{}", &show(&digest)[..2], &show(&digest)[2..])
    );
}

#[test]
fn three_paths_share_a_directory_only_when_their_prefix_does() {
    let first = path(&of(b"a"));
    let second = path(&of(b"b"));

    assert!(first.starts_with("objects/"));
    assert_eq!(first.split('/').count(), 3);
    // Different content, different digest; nothing else is promised here.
    assert_ne!(first, second);
}

#[test]
fn verification_passes_for_the_bytes_the_digest_names() {
    let bytes = b"let change = self.clamp();\n";

    assert_eq!(verify(&path(&of(bytes)), bytes, &of(bytes)), Ok(()));
}

#[test]
fn one_flipped_byte_is_corrupt_and_names_both_digests() {
    let bytes = b"let change = self.clamp();\n";
    let expected = of(bytes);
    let mut corrupted = bytes.to_vec();
    corrupted[4] ^= 0b0000_0001;

    let error = verify(&path(&expected), &corrupted, &expected).expect_err("corrupt");

    assert_eq!(error.expected, expected);
    assert_eq!(error.actual, of(&corrupted));
    assert_ne!(error.expected, error.actual);
}

#[test]
fn the_corruption_message_is_the_one_an_error_catalogue_looks_for() {
    let bytes = b"x";
    let expected = of(bytes);
    let mut corrupted = bytes.to_vec();
    corrupted[0] = b'y';

    let error = verify("objects/ab/cdef", &corrupted, &expected).expect_err("corrupt");

    assert!(
        error
            .to_string()
            .starts_with("env.corrupt: objects/ab/cdef holds "),
        "{error}"
    );
    assert!(error.to_string().contains(&show(&expected)), "{error}");
}

#[test]
fn an_empty_blob_is_storable() {
    let empty = b"";

    assert_eq!(verify(&path(&of(empty)), empty, &of(empty)), Ok(()));
    assert_eq!(show(&of(empty)).len(), 64);
}

#[test]
fn a_digest_parses_back_from_its_printed_form() {
    let digest = of(b"round trip");

    assert_eq!(parse(&show(&digest)), Some(digest));
    assert_eq!(parse("not a digest"), None);
}
