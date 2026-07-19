use loom_core::container::metadata_tags::MetadataTags;
use loom_core::container::session::{
    decode_session_full, encode_session, update_session_tags, update_session_tags_inplace,
};

#[test]
fn test_metadata_inplace_update() {
    let tracks = vec![vec![vec![0i64; 1000]]];
    let track_names = vec!["track1".to_string()];

    let mut session_bytes =
        encode_session(&tracks, &track_names, 44100, 16, 256).expect("Failed to encode session");

    let (_, _, _, tags_orig, _) =
        decode_session_full(&session_bytes).expect("Failed to decode session");
    assert!(tags_orig.is_none(), "Should have no tags initially");

    let mut new_tags = MetadataTags::new();
    new_tags.add_tag("artist".to_string(), "John Doe".to_string());
    new_tags.add_tag("title".to_string(), "Silent Song".to_string());

    update_session_tags_inplace(&mut session_bytes, &new_tags)
        .expect("Failed to update tags in-place");

    let (_, _, _, tags_updated, _) =
        decode_session_full(&session_bytes).expect("Failed to decode session");
    assert!(
        tags_updated.is_some(),
        "Tags should be present after update"
    );
    let tags_updated = tags_updated.unwrap();
    assert_eq!(tags_updated.tags.get("artist").unwrap(), "John Doe");
    assert_eq!(tags_updated.tags.get("title").unwrap(), "Silent Song");

    let mut small_tags = MetadataTags::new();
    small_tags.add_tag("artist".to_string(), "JD".to_string());

    update_session_tags_inplace(&mut session_bytes, &small_tags)
        .expect("Failed to shrink tags in-place");

    let (_, _, _, tags_small, _) =
        decode_session_full(&session_bytes).expect("Failed to decode session");
    assert_eq!(tags_small.unwrap().tags.get("artist").unwrap(), "JD");

    let mut huge_tags = MetadataTags::new();
    huge_tags.add_tag("lyrics".to_string(), "a".repeat(5000));

    let res = update_session_tags_inplace(&mut session_bytes, &huge_tags);
    assert!(
        res.is_err(),
        "In-place update should fail when exceeding padding space"
    );

    let new_session_bytes =
        update_session_tags(&session_bytes, huge_tags).expect("Failed full rewrite");

    let (_, _, _, tags_huge, _) =
        decode_session_full(&new_session_bytes).expect("Failed to decode session");
    assert_eq!(
        tags_huge.unwrap().tags.get("lyrics").unwrap(),
        &"a".repeat(5000)
    );
}
