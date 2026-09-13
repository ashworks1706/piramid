#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "assertions in tests"
)]

//! Bytes written by bincode 1.3 decode with the storage codec and re-encode byte for byte.
//!
//! The files under tests/fixtures/bincode1 were written by bincode 1.3.3 with bincode::serialize.

use std::collections::HashMap;
use std::path::PathBuf;

use piramid_core::config::{CollectionConfig, SearchConfig};
use piramid_core::metadata::{Metadata, MetadataValue};
use piramid_core::Document;
use piramid_database::index::{
    load_vector_index, HashMapVectorReader, IndexSearchRequest, IndexType, SerializableIndex,
};
use piramid_database::storage::codec;
use piramid_database::storage::record_store::RecordStore;
use piramid_database::storage::sidecars::{EntryPointer, SidecarManager};
use piramid_database::storage::CollectionMetadata;
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
    u32_le(&mut expected, 1);
    u64_le(&mut expected, 6);
    expected.extend_from_slice(b"legacy");
    u64_le(&mut expected, 1_700_000_000);
    u64_le(&mut expected, 1_700_000_500);
    expected.push(1);
    u64_le(&mut expected, 3);
    u64_le(&mut expected, 1);

    assert_eq!(expected, fixture("manifest.bin"));

    let metadata: CollectionMetadata = codec::decode(&expected).unwrap();
    assert_eq!(metadata.schema_version, 1);
    assert_eq!(metadata.name, "legacy");
    assert_eq!(metadata.created_at, 1_700_000_000);
    assert_eq!(metadata.updated_at, 1_700_000_500);
    assert_eq!(metadata.dimensions, Some(3));
    assert_eq!(metadata.vector_count, 1);
    assert_eq!(codec::encode(&metadata).unwrap(), expected);
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
fn every_index_family_round_trips_byte_identical() {
    for (name, index_type) in [
        ("flat.bin", IndexType::Flat),
        ("hnsw.bin", IndexType::Hnsw),
        ("ivf.bin", IndexType::Ivf),
    ] {
        let bytes = fixture(name);
        let serializable: SerializableIndex = codec::decode(&bytes).unwrap();
        assert_eq!(codec::encode(&serializable).unwrap(), bytes, "{name}");

        let index = serializable.to_trait_object();
        assert_eq!(index.index_type(), index_type, "{name}");
        assert_eq!(index.stats().total_vectors, 1, "{name}");
    }
}

#[test]
fn sidecars_written_by_bincode1_load() {
    let base = scratch_base("legacy_sidecars");
    let sidecars = SidecarManager::at(&base);
    std::fs::write(sidecars.offsets_path(), fixture("offsets.bin")).unwrap();
    std::fs::write(sidecars.manifest_path(), fixture("manifest.bin")).unwrap();
    std::fs::write(sidecars.vector_index_path(), fixture("hnsw.bin")).unwrap();

    let offsets = sidecars.load_offsets().unwrap();
    assert_eq!(offsets[&Uuid::from_u128(ID)].offset, 4096);

    let manifest = sidecars.load_manifest().unwrap().unwrap();
    assert_eq!(manifest.name, "legacy");

    let index = load_vector_index(&base).unwrap().unwrap();
    let vectors = HashMap::from([(Uuid::from_u128(ID), vec![1.0f32, 0.0, 0.0])]);
    let reader = HashMapVectorReader::new(&vectors);
    let no_metadata: HashMap<Uuid, Metadata> = HashMap::new();
    let hits = index
        .search(IndexSearchRequest::new(
            &[1.0, 0.0, 0.0],
            1,
            &reader,
            SearchConfig::default(),
            &no_metadata,
        ))
        .unwrap();
    assert_eq!(hits, vec![Uuid::from_u128(ID)]);
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
