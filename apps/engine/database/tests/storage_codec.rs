#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "assertions in tests"
)]

//! Offsets and documents written by bincode 1.3 decode with the storage codec and re-encode byte
//! for byte, and a manifest written by bincode 1.3 is schema 1 and refused.
//!
//! The files under tests/fixtures/bincode1 were written by bincode 1.3.3 with bincode::serialize.

use std::collections::HashMap;
use std::path::PathBuf;

use piramid_core::config::CollectionConfig;
use piramid_core::metadata::{Metadata, MetadataValue};
use piramid_core::Document;
use piramid_database::storage::codec;
use piramid_database::storage::record_store::RecordStore;
use piramid_database::storage::sidecars::{EntryPointer, SidecarManager};
use piramid_database::storage::CollectionMetadata;
use piramid_hardware::compute::Metric;
use uuid::Uuid;

const ID: u128 = 0x0123_4567_89ab_cdef_fedc_ba98_7654_3210;

fn fixture(name: &str) -> Vec<u8> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/bincode1")
        .join(name);
    std::fs::read(path).unwrap()
}

fn scratch_base(name: &str) -> String {
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("storage_codec");
    std::fs::create_dir_all(&dir).unwrap();
    let base = dir.join(name).to_string_lossy().to_string();
    for path in SidecarManager::at(&base).all_paths() {
        let _ = std::fs::remove_file(path);
    }
    let _ = std::fs::remove_file(&base);
    base
}

fn u32_le(out: &mut Vec<u8>, v: u32) {
    out.extend_from_slice(&v.to_le_bytes());
}

fn u64_le(out: &mut Vec<u8>, v: u64) {
    out.extend_from_slice(&v.to_le_bytes());
}

#[test]
fn manifest_layout_is_fixed_width_little_endian() {
    let mut expected = Vec::new();
    u32_le(&mut expected, 2);
    u64_le(&mut expected, 7);
    expected.extend_from_slice(b"current");
    u64_le(&mut expected, 1_700_000_000);
    u64_le(&mut expected, 1_700_000_500);
    expected.push(1);
    u64_le(&mut expected, 3);
    u64_le(&mut expected, 1);
    u32_le(&mut expected, 2);

    let metadata = CollectionMetadata::decode(&expected).unwrap();
    assert_eq!(metadata.schema_version, 2);
    assert_eq!(metadata.name, "current");
    assert_eq!(metadata.created_at, 1_700_000_000);
    assert_eq!(metadata.updated_at, 1_700_000_500);
    assert_eq!(metadata.dimensions, Some(3));
    assert_eq!(metadata.vector_count, 1);
    assert_eq!(metadata.metric, Metric::DotProduct);
    assert_eq!(codec::encode(&metadata).unwrap(), expected);
}

#[test]
fn a_manifest_written_by_bincode1_is_schema_1_and_refused() {
    let error = CollectionMetadata::decode(&fixture("manifest.bin")).unwrap_err();

    assert!(
        matches!(
            &error,
            piramid_core::error::PiramidError::Storage(
                piramid_core::error::StorageError::LegacyManifest { collection }
            ) if collection == "legacy"
        ),
        "{error}"
    );
}

#[test]
fn a_manifest_of_an_unknown_schema_version_is_unsupported() {
    let mut manifest = CollectionMetadata::new("future".to_string(), Metric::Cosine).unwrap();
    manifest.schema_version = 3;
    let error = CollectionMetadata::decode(&codec::encode(&manifest).unwrap()).unwrap_err();

    assert!(
        matches!(
            &error,
            piramid_core::error::PiramidError::Storage(
                piramid_core::error::StorageError::UnsupportedManifest { collection, found: 3 }
            ) if collection == "future"
        ),
        "{error}"
    );
}

#[test]
fn offsets_layout_is_fixed_width_little_endian() {
    let mut expected = Vec::new();
    u64_le(&mut expected, 1);
    u64_le(&mut expected, 16);
    expected.extend_from_slice(&ID.to_be_bytes());
    u64_le(&mut expected, 4096);
    u32_le(&mut expected, 123);

    assert_eq!(expected, fixture("offsets.bin"));

    let offsets: HashMap<Uuid, EntryPointer> = codec::decode(&expected).unwrap();
    let pointer = &offsets[&Uuid::from_u128(ID)];
    assert_eq!((pointer.offset, pointer.length), (4096, 123));
    assert_eq!(codec::encode(&offsets).unwrap(), expected);
}

#[test]
fn document_round_trips_byte_identical() {
    let bytes = fixture("document.bin");
    let document: Document = codec::decode(&bytes).unwrap();

    assert_eq!(document.id, Uuid::from_u128(ID));
    assert_eq!(document.vector, vec![1.0, -2.5, 0.25]);
    assert_eq!(document.text, "hello");
    let expected: Metadata = HashMap::from([(
        "tags".to_string(),
        MetadataValue::Array(vec![
            MetadataValue::String("a".into()),
            MetadataValue::Integer(-7),
            MetadataValue::Float(1.5),
            MetadataValue::Boolean(true),
            MetadataValue::Null,
        ]),
    )]);
    assert_eq!(document.metadata, expected);
    assert_eq!(RecordStore::encode_document(&document).unwrap(), bytes);
}

#[test]
fn sidecars_written_by_bincode1_load() {
    let base = scratch_base("legacy_sidecars");
    let sidecars = SidecarManager::at(&base);
    std::fs::write(sidecars.offsets_path(), fixture("offsets.bin")).unwrap();
    std::fs::write(sidecars.manifest_path(), fixture("manifest.bin")).unwrap();

    let offsets = sidecars.load_offsets().unwrap();
    assert_eq!(offsets[&Uuid::from_u128(ID)].offset, 4096);

    let error = sidecars.load_manifest().unwrap_err();
    assert!(error.to_string().contains("'legacy'"), "{error}");
}

#[test]
fn record_written_by_bincode1_reads() {
    let base = scratch_base("legacy_record");
    let mut config = CollectionConfig::default();
    config.memory.use_mmap = false;
    let mut store = RecordStore::open(&base, &config, &HashMap::new()).unwrap();
    let pointer = store.append(&fixture("document.bin")).unwrap();

    let document = store.read_document(&pointer).unwrap();
    assert_eq!(document.id, Uuid::from_u128(ID));
    assert_eq!(document.text, "hello");
}

#[test]
fn a_pointer_outside_the_mapping_is_corruption() {
    let base = scratch_base("outside_mapping");
    let mut config = CollectionConfig::default();
    config.memory.use_mmap = true;
    config.memory.initial_mmap_size = 4096;
    let store = RecordStore::open(&base, &config, &HashMap::new()).unwrap();
    let mapped = store.mapped_len().unwrap() as u64;

    let error = store
        .read_document(&EntryPointer::new(mapped, 16))
        .unwrap_err();

    assert!(error.to_string().contains("outside the"), "{error}");
}

#[test]
fn trailing_bytes_after_a_value_are_corruption() {
    let mut bytes = codec::encode(&7u64).unwrap();
    assert_eq!(codec::decode::<u64>(&bytes).unwrap(), 7);

    bytes.extend_from_slice(&[0, 0]);
    let error = codec::decode::<u64>(&bytes).unwrap_err();

    assert!(error.to_string().contains("2 trailing bytes"), "{error}");
}

#[test]
fn a_record_longer_than_a_pointer_can_address_is_refused() {
    use piramid_database::storage::record_store::record_length;

    assert_eq!(record_length(u32::MAX as usize).unwrap(), u32::MAX);
    let error = record_length(u32::MAX as usize + 1).unwrap_err();
    assert!(error.to_string().contains("exceeds 4 GiB"), "{error}");
}
