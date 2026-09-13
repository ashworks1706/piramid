#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "assertions in tests"
)]
//! Collections: storage, persistence, WAL, checkpoints and index growth.

use std::fs;
use {
    piramid_core::config::CacheConfig,
    piramid_core::config::CollectionConfig,
    piramid_core::config::MemoryConfig,
    piramid_core::metadata::metadata,
    piramid_core::Document,
    piramid_database::search::SearchParams,
    piramid_database::Collection,
    piramid_database::{compact, CollectionOpenOptions},
    piramid_hardware::compute::Metric,
};

fn ensure_test_dir() {
    let _ = fs::create_dir_all(env!("CARGO_TARGET_TMPDIR"));
}

fn cleanup_test_files(paths: &[&str]) {
    ensure_test_dir();
    for path in paths {
        let _ = fs::remove_file(path);
    }
}

#[test]
fn basic_store_and_retrieve() {
    ensure_test_dir();
    let test_path = concat!(env!("CARGO_TARGET_TMPDIR"), "/test_basic.db");
    let files = vec![
        test_path,
        concat!(env!("CARGO_TARGET_TMPDIR"), "/test_basic.db.offsets.db"),
        concat!(env!("CARGO_TARGET_TMPDIR"), "/test_basic.db.wal.db"),
        concat!(env!("CARGO_TARGET_TMPDIR"), "/test_basic.db.vecindex.db"),
        concat!(env!("CARGO_TARGET_TMPDIR"), "/test_basic.db.manifest.db"),
    ];
    cleanup_test_files(&files);

    let mut storage = Collection::open(test_path).unwrap();
    let entry = Document::new(vec![1.0, 2.0, 3.0], "test".to_string());
    let id = storage.insert(entry).unwrap();

    let retrieved = storage.get(&id).unwrap().unwrap();
    assert_eq!(retrieved.text, "test");
    assert_eq!(retrieved.vector(), vec![1.0, 2.0, 3.0]);

    drop(storage);
    cleanup_test_files(&files);
}

#[test]
fn persistence_roundtrip() {
    ensure_test_dir();
    let test_path = concat!(env!("CARGO_TARGET_TMPDIR"), "/test_persist.db");
    let files = vec![
        test_path,
        concat!(env!("CARGO_TARGET_TMPDIR"), "/test_persist.db.offsets.db"),
        concat!(env!("CARGO_TARGET_TMPDIR"), "/test_persist.db.wal.db"),
        concat!(env!("CARGO_TARGET_TMPDIR"), "/test_persist.db.vecindex.db"),
        concat!(env!("CARGO_TARGET_TMPDIR"), "/test_persist.db.manifest.db"),
    ];
    cleanup_test_files(&files);

    let id1;
    let id2;
    {
        let mut storage = Collection::open(test_path).unwrap();
        id1 = storage
            .insert(Document::new(vec![1.0, 2.0], "first".into()))
            .unwrap();
        id2 = storage
            .insert(Document::new(vec![3.0, 4.0], "second".into()))
            .unwrap();
    }

    {
        let storage = Collection::open(test_path).unwrap();
        assert_eq!(storage.count(), 2);
        assert_eq!(storage.get(&id1).unwrap().unwrap().text, "first");
        assert_eq!(storage.get(&id2).unwrap().unwrap().text, "second");
    }

    cleanup_test_files(&files);
}

#[test]
fn search_returns_results() {
    ensure_test_dir();
    let test_path = concat!(env!("CARGO_TARGET_TMPDIR"), "/test_search.db");
    let files = vec![
        test_path,
        concat!(env!("CARGO_TARGET_TMPDIR"), "/test_search.db.offsets.db"),
        concat!(env!("CARGO_TARGET_TMPDIR"), "/test_search.db.wal.db"),
        concat!(env!("CARGO_TARGET_TMPDIR"), "/test_search.db.vecindex.db"),
        concat!(env!("CARGO_TARGET_TMPDIR"), "/test_search.db.manifest.db"),
    ];
    cleanup_test_files(&files);

    let mut storage = Collection::open(test_path).unwrap();
    let vectors = [
        vec![1.0, 0.0, 0.0],
        vec![0.0, 1.0, 0.0],
        vec![0.0, 0.0, 1.0],
        vec![0.9, 0.1, 0.0],
    ];
    for (i, vec) in vectors.iter().enumerate() {
        storage
            .insert(Document::new(vec.clone(), format!("vec{i}")))
            .unwrap();
    }

    let params = SearchParams::default();
    let results = storage
        .search(&[1.0, 0.0, 0.0], 2, Metric::Cosine, params)
        .unwrap();
    assert_eq!(results.len(), 2);
    assert_eq!(results[0].document.text, "vec0");

    drop(storage);
    cleanup_test_files(&files);
}

#[test]
fn batch_search_multi_queries() {
    ensure_test_dir();
    let test_path = concat!(env!("CARGO_TARGET_TMPDIR"), "/test_batch_search.db");
    let files = vec![
        test_path,
        concat!(
            env!("CARGO_TARGET_TMPDIR"),
            "/test_batch_search.db.offsets.db"
        ),
        concat!(env!("CARGO_TARGET_TMPDIR"), "/test_batch_search.db.wal.db"),
        concat!(
            env!("CARGO_TARGET_TMPDIR"),
            "/test_batch_search.db.vecindex.db"
        ),
        concat!(
            env!("CARGO_TARGET_TMPDIR"),
            "/test_batch_search.db.manifest.db"
        ),
    ];
    cleanup_test_files(&files);

    let mut storage = Collection::open(test_path).unwrap();
    for i in 0..10 {
        storage
            .insert(Document::new(
                vec![i as f32 + 1.0, 0.0, 0.0],
                format!("vec{i}"),
            ))
            .unwrap();
    }

    let queries = vec![
        vec![1.0, 0.0, 0.0],
        vec![5.0, 0.0, 0.0],
        vec![9.0, 0.0, 0.0],
    ];
    let results = storage
        .search_batch_with(&queries, 2, Metric::Cosine, SearchParams::default())
        .unwrap();
    assert_eq!(results.len(), 3);
    assert!(results.iter().all(|hits| !hits.is_empty()));

    drop(storage);
    cleanup_test_files(&files);
}

#[test]
fn no_mmap_insert_grows_file_without_panicking() {
    ensure_test_dir();
    let test_path = concat!(env!("CARGO_TARGET_TMPDIR"), "/test_no_mmap_grow.db");
    let files = vec![
        test_path,
        concat!(
            env!("CARGO_TARGET_TMPDIR"),
            "/test_no_mmap_grow.db.offsets.db"
        ),
        concat!(env!("CARGO_TARGET_TMPDIR"), "/test_no_mmap_grow.db.wal.db"),
        concat!(
            env!("CARGO_TARGET_TMPDIR"),
            "/test_no_mmap_grow.db.wal.meta"
        ),
        concat!(
            env!("CARGO_TARGET_TMPDIR"),
            "/test_no_mmap_grow.db.vecindex.db"
        ),
        concat!(
            env!("CARGO_TARGET_TMPDIR"),
            "/test_no_mmap_grow.db.manifest.db"
        ),
    ];
    cleanup_test_files(&files);

    let config = CollectionConfig {
        memory: MemoryConfig::no_mmap(),
        ..CollectionConfig::default()
    };

    let mut storage =
        Collection::open_with_options(test_path, CollectionOpenOptions { config }).unwrap();
    let vector = vec![0.25; 1_100_000];
    let id = storage
        .insert(Document::new(
            vector.clone(),
            "large no-mmap document".to_string(),
        ))
        .unwrap();

    let retrieved = storage.get(&id).unwrap().unwrap();
    assert_eq!(retrieved.text, "large no-mmap document");
    assert_eq!(retrieved.vector().len(), vector.len());

    drop(storage);
    cleanup_test_files(&files);
}

#[test]
fn updates_write_one_wal_entry_each() {
    ensure_test_dir();
    let test_path = concat!(env!("CARGO_TARGET_TMPDIR"), "/test_update_wal.db");
    let files = vec![
        test_path,
        concat!(
            env!("CARGO_TARGET_TMPDIR"),
            "/test_update_wal.db.offsets.db"
        ),
        concat!(env!("CARGO_TARGET_TMPDIR"), "/test_update_wal.db.wal.db"),
        concat!(
            env!("CARGO_TARGET_TMPDIR"),
            "/test_update_wal.db.vecindex.db"
        ),
        concat!(
            env!("CARGO_TARGET_TMPDIR"),
            "/test_update_wal.db.manifest.db"
        ),
    ];
    cleanup_test_files(&files);

    let mut storage = Collection::open(test_path).unwrap();
    let id = storage
        .insert(Document::with_metadata(
            vec![1.0, 2.0, 3.0],
            "original".to_string(),
            metadata([("kind", "initial".into())]),
        ))
        .unwrap();

    storage
        .update_metadata(&id, metadata([("kind", "updated".into())]))
        .unwrap();
    storage.update_vector(&id, vec![3.0, 2.0, 1.0]).unwrap();
    assert_eq!(
        storage.get(&id).unwrap().unwrap().vector(),
        vec![3.0, 2.0, 1.0]
    );

    let wal = fs::read_to_string(format!("{test_path}.wal.db")).unwrap();
    assert_eq!(
        wal.lines()
            .filter(|line| line.contains("\"Insert\""))
            .count(),
        1
    );
    assert_eq!(
        wal.lines()
            .filter(|line| line.contains("\"Update\""))
            .count(),
        2
    );
    assert_eq!(
        wal.lines()
            .filter(|line| line.contains("\"Delete\""))
            .count(),
        0
    );

    drop(storage);
    cleanup_test_files(&files);
}

#[test]
fn update_vector_persists_new_raw_vector_after_reopen() {
    ensure_test_dir();
    let test_path = concat!(
        env!("CARGO_TARGET_TMPDIR"),
        "/test_update_vector_persist.db"
    );
    let files = vec![
        test_path,
        concat!(
            env!("CARGO_TARGET_TMPDIR"),
            "/test_update_vector_persist.db.offsets.db"
        ),
        concat!(
            env!("CARGO_TARGET_TMPDIR"),
            "/test_update_vector_persist.db.wal.db"
        ),
        concat!(
            env!("CARGO_TARGET_TMPDIR"),
            "/test_update_vector_persist.db.vecindex.db"
        ),
        concat!(
            env!("CARGO_TARGET_TMPDIR"),
            "/test_update_vector_persist.db.manifest.db"
        ),
        concat!(
            env!("CARGO_TARGET_TMPDIR"),
            "/test_update_vector_persist.db.wal.meta"
        ),
    ];
    cleanup_test_files(&files);

    let id = {
        let mut storage = Collection::open(test_path).unwrap();
        let id = storage
            .insert(Document::new(vec![1.0, 2.0, 3.0], "updated".into()))
            .unwrap();
        storage.update_vector(&id, vec![3.0, 2.0, 1.0]).unwrap();
        id
    };

    let storage = Collection::open(test_path).unwrap();
    assert_eq!(
        storage.get(&id).unwrap().unwrap().vector(),
        vec![3.0, 2.0, 1.0]
    );

    drop(storage);
    cleanup_test_files(&files);
}

#[test]
fn sidecar_files_persist_at_checkpoint_only() {
    ensure_test_dir();
    let test_path = concat!(env!("CARGO_TARGET_TMPDIR"), "/test_checkpoint_only.db");
    let files = vec![
        test_path,
        concat!(
            env!("CARGO_TARGET_TMPDIR"),
            "/test_checkpoint_only.db.offsets.db"
        ),
        concat!(
            env!("CARGO_TARGET_TMPDIR"),
            "/test_checkpoint_only.db.wal.db"
        ),
        concat!(
            env!("CARGO_TARGET_TMPDIR"),
            "/test_checkpoint_only.db.vecindex.db"
        ),
        concat!(
            env!("CARGO_TARGET_TMPDIR"),
            "/test_checkpoint_only.db.manifest.db"
        ),
        concat!(
            env!("CARGO_TARGET_TMPDIR"),
            "/test_checkpoint_only.db.wal.meta"
        ),
    ];
    cleanup_test_files(&files);

    let mut storage = Collection::open(test_path).unwrap();
    storage
        .insert(Document::new(vec![1.0, 2.0, 3.0], "checkpoint only".into()))
        .unwrap();

    assert!(fs::metadata(format!("{test_path}.offsets.db")).is_err());
    assert!(fs::metadata(format!("{test_path}.vecindex.db")).is_err());

    storage.checkpoint().unwrap();

    assert!(fs::metadata(format!("{test_path}.offsets.db")).is_ok());
    assert!(fs::metadata(format!("{test_path}.vecindex.db")).is_ok());

    drop(storage);
    cleanup_test_files(&files);
}

#[test]
fn metadata_cache_is_bounded_without_evicting_vectors() {
    ensure_test_dir();
    let test_path = concat!(env!("CARGO_TARGET_TMPDIR"), "/test_cache_manager_bounds.db");
    let files = vec![
        test_path,
        concat!(
            env!("CARGO_TARGET_TMPDIR"),
            "/test_cache_manager_bounds.db.offsets.db"
        ),
        concat!(
            env!("CARGO_TARGET_TMPDIR"),
            "/test_cache_manager_bounds.db.wal.db"
        ),
        concat!(
            env!("CARGO_TARGET_TMPDIR"),
            "/test_cache_manager_bounds.db.vecindex.db"
        ),
        concat!(
            env!("CARGO_TARGET_TMPDIR"),
            "/test_cache_manager_bounds.db.manifest.db"
        ),
        concat!(
            env!("CARGO_TARGET_TMPDIR"),
            "/test_cache_manager_bounds.db.wal.meta"
        ),
    ];
    cleanup_test_files(&files);

    let config = CollectionConfig {
        cache: CacheConfig::with_size(1),
        ..CollectionConfig::default()
    };
    let mut storage =
        Collection::open_with_options(test_path, CollectionOpenOptions { config }).unwrap();

    let id_a = storage
        .insert(Document::with_metadata(
            vec![1.0, 0.0, 0.0],
            "first".to_string(),
            metadata([("kind", "a".into())]),
        ))
        .unwrap();
    let id_b = storage
        .insert(Document::with_metadata(
            vec![0.0, 1.0, 0.0],
            "second".to_string(),
            metadata([("kind", "b".into())]),
        ))
        .unwrap();

    assert_eq!(storage.vector_reader().len(), 2);
    assert_eq!(storage.metadata_view().len(), 1);
    assert!(storage.vector_reader().get(&id_a).is_some());
    assert!(storage.vector_reader().get(&id_b).is_some());

    drop(storage);
    cleanup_test_files(&files);
}

#[test]
fn append_cursor_survives_reopen_and_preserves_existing_records() {
    ensure_test_dir();
    let test_path = concat!(env!("CARGO_TARGET_TMPDIR"), "/test_append_cursor_reopen.db");
    let files = vec![
        test_path,
        concat!(
            env!("CARGO_TARGET_TMPDIR"),
            "/test_append_cursor_reopen.db.offsets.db"
        ),
        concat!(
            env!("CARGO_TARGET_TMPDIR"),
            "/test_append_cursor_reopen.db.wal.db"
        ),
        concat!(
            env!("CARGO_TARGET_TMPDIR"),
            "/test_append_cursor_reopen.db.vecindex.db"
        ),
        concat!(
            env!("CARGO_TARGET_TMPDIR"),
            "/test_append_cursor_reopen.db.manifest.db"
        ),
        concat!(
            env!("CARGO_TARGET_TMPDIR"),
            "/test_append_cursor_reopen.db.wal.meta"
        ),
    ];
    cleanup_test_files(&files);

    let first_id = {
        let mut storage = Collection::open(test_path).unwrap();
        let first_id = storage
            .insert(Document::new(vec![1.0, 0.0, 0.0], "first".to_string()))
            .unwrap();
        storage.checkpoint().unwrap();
        first_id
    };

    let second_id = {
        let mut storage = Collection::open(test_path).unwrap();
        storage
            .insert(Document::new(vec![0.0, 1.0, 0.0], "second".to_string()))
            .unwrap()
    };

    let storage = Collection::open(test_path).unwrap();
    assert_eq!(storage.count(), 2);
    assert_eq!(storage.get(&first_id).unwrap().unwrap().text, "first");
    assert_eq!(storage.get(&second_id).unwrap().unwrap().text, "second");

    drop(storage);
    cleanup_test_files(&files);
}

#[test]
fn compaction_rewrites_live_records_through_temp_record_store() {
    ensure_test_dir();
    let test_path = concat!(env!("CARGO_TARGET_TMPDIR"), "/test_record_store_compact.db");
    let files = vec![
        test_path,
        concat!(
            env!("CARGO_TARGET_TMPDIR"),
            "/test_record_store_compact.db.offsets.db"
        ),
        concat!(
            env!("CARGO_TARGET_TMPDIR"),
            "/test_record_store_compact.db.wal.db"
        ),
        concat!(
            env!("CARGO_TARGET_TMPDIR"),
            "/test_record_store_compact.db.vecindex.db"
        ),
        concat!(
            env!("CARGO_TARGET_TMPDIR"),
            "/test_record_store_compact.db.manifest.db"
        ),
        concat!(
            env!("CARGO_TARGET_TMPDIR"),
            "/test_record_store_compact.db.wal.meta"
        ),
        concat!(
            env!("CARGO_TARGET_TMPDIR"),
            "/test_record_store_compact.db.compact"
        ),
    ];
    cleanup_test_files(&files);

    let mut storage = Collection::open(test_path).unwrap();
    let keep_id = storage
        .insert(Document::new(vec![1.0, 0.0, 0.0], "keep".to_string()))
        .unwrap();
    let delete_id = storage
        .insert(Document::new(vec![0.0, 1.0, 0.0], "delete".to_string()))
        .unwrap();
    storage.delete(&delete_id).unwrap();

    let stats = compact(&mut storage).unwrap();

    assert_eq!(stats.documents, 1);
    assert!(
        stats.bytes_after < stats.bytes_before,
        "the deleted record is reclaimed: {stats:?}"
    );
    assert_eq!(storage.count(), 1);
    assert_eq!(storage.get(&keep_id).unwrap().unwrap().text, "keep");
    assert!(storage.get(&delete_id).unwrap().is_none());
    assert!(fs::metadata(format!("{test_path}.compact")).is_err());

    drop(storage);
    cleanup_test_files(&files);
}

// A collection with sync_on_write enabled accepts writes and replays them on reopen.
#[test]
fn writes_replay_with_sync_on_write_enabled() {
    use piramid_core::config::CollectionConfig;

    let path = concat!(env!("CARGO_TARGET_TMPDIR"), "/test_wal_sync.db");
    for suffix in [
        "",
        ".offsets.db",
        ".wal.db",
        ".vecindex.db",
        ".manifest.db",
        ".wal.meta",
    ] {
        let _ = fs::remove_file(format!("{path}{suffix}"));
    }

    let mut config = CollectionConfig::default();
    config.wal.sync_on_write = true;
    // No checkpoint fires, so reopening depends on the WAL alone.
    config.wal.checkpoint_frequency = 10_000;

    {
        let mut collection = Collection::open_with_options(path, config.clone().into()).unwrap();
        collection
            .insert(Document::new(vec![1.0, 0.0], "durable".to_string()))
            .unwrap();
    }

    let reopened = Collection::open_with_options(path, config.into()).unwrap();
    assert_eq!(
        reopened.count(),
        1,
        "the synced write must replay from the WAL"
    );
}

// A WAL that grows past max_log_size triggers a checkpoint.
#[test]
fn a_wal_past_max_log_size_triggers_a_checkpoint() {
    use piramid_core::config::CollectionConfig;

    let path = concat!(env!("CARGO_TARGET_TMPDIR"), "/test_wal_max_size.db");
    for suffix in [
        "",
        ".offsets.db",
        ".wal.db",
        ".vecindex.db",
        ".manifest.db",
        ".wal.meta",
    ] {
        let _ = fs::remove_file(format!("{path}{suffix}"));
    }

    let mut config = CollectionConfig::default();
    config.wal.checkpoint_frequency = 10_000; // never fires on count
    config.wal.max_log_size = 256; // a few entries' worth

    let mut collection = Collection::open_with_options(path, config.into()).unwrap();
    for i in 0..12 {
        collection
            .insert(Document::new(vec![i as f32 + 1.0, 0.0], format!("doc{i}")))
            .unwrap();
    }

    // A checkpoint rotates the log, so it cannot still be above the bound.
    let wal_len = fs::metadata(format!("{path}.wal.db")).unwrap().len();
    assert!(
        wal_len < 256 * 4,
        "the size trigger never fired; WAL is {wal_len} bytes"
    );
    assert_eq!(collection.count(), 12);
}

#[test]
fn a_collection_hands_its_vectors_over_as_one_slab() {
    ensure_test_dir();
    let test_path = concat!(env!("CARGO_TARGET_TMPDIR"), "/test_slab.db");
    let files = vec![
        test_path,
        concat!(env!("CARGO_TARGET_TMPDIR"), "/test_slab.db.offsets.db"),
        concat!(env!("CARGO_TARGET_TMPDIR"), "/test_slab.db.wal.db"),
        concat!(env!("CARGO_TARGET_TMPDIR"), "/test_slab.db.vecindex.db"),
        concat!(env!("CARGO_TARGET_TMPDIR"), "/test_slab.db.manifest.db"),
    ];
    cleanup_test_files(&files);

    let mut storage = Collection::open(test_path).unwrap();

    let ids: Vec<_> = (0..4)
        .map(|i| {
            storage
                .insert(Document::new(vec![i as f32, 1.0, 2.0], format!("doc {i}")))
                .unwrap()
        })
        .collect();

    // The reader is contiguous.
    let slab = storage
        .vector_reader()
        .as_slab()
        .expect("a collection with no deletes is contiguous");
    assert_eq!(slab.dim, 3);
    assert_eq!(slab.data.len(), 4 * 3);
    assert_eq!(slab.rows(), 4);

    // A delete withdraws the slab fast path until an insert reuses the hole.
    storage.delete(&ids[1]).unwrap();
    assert!(storage.vector_reader().as_slab().is_none());

    storage
        .insert(Document::new(vec![9.0, 9.0, 9.0], "refill".to_string()))
        .unwrap();
    let slab = storage
        .vector_reader()
        .as_slab()
        .expect("the hole was reused, so the slab is whole again");
    assert_eq!(slab.data.len(), 4 * 3);

    drop(storage);
    cleanup_test_files(&files);
}

// The interval trigger counts from the open, so it fires before any other trigger has run a
// first checkpoint.
#[test]
fn the_checkpoint_interval_runs_from_the_open() {
    use piramid_core::config::CollectionConfig;

    let path = concat!(env!("CARGO_TARGET_TMPDIR"), "/test_wal_interval.db");
    for suffix in [
        "",
        ".offsets.db",
        ".wal.db",
        ".vecindex.db",
        ".manifest.db",
        ".wal.meta",
    ] {
        let _ = fs::remove_file(format!("{path}{suffix}"));
    }

    let mut config = CollectionConfig::default();
    config.wal.checkpoint_frequency = 10_000;
    config.wal.checkpoint_interval_secs = Some(0);

    let mut collection = Collection::open_with_options(path, config.into()).unwrap();
    assert_eq!(collection.checkpoint.last_checkpoint(), None);
    collection
        .insert(Document::new(vec![1.0, 0.0], "doc".to_string()))
        .unwrap();
    assert!(collection.checkpoint.last_checkpoint().is_some());
}

// An auto index moves to the family its thresholds name as the collection grows, and every
// vector stays searchable across each move.
#[test]
fn an_auto_index_grows_into_the_family_its_size_picks() {
    use piramid_core::config::{AutoIndexConfig, CollectionConfig, IndexConfig};
    use piramid_database::index::IndexType;
    use piramid_database::search::SearchParams;
    use piramid_hardware::compute::Metric;

    let path = concat!(env!("CARGO_TARGET_TMPDIR"), "/test_auto_index_growth.db");
    for suffix in [
        "",
        ".offsets.db",
        ".wal.db",
        ".vecindex.db",
        ".manifest.db",
        ".wal.meta",
    ] {
        let _ = fs::remove_file(format!("{path}{suffix}"));
    }
    let config = CollectionConfig {
        index: IndexConfig::Auto {
            metric: Metric::Cosine,
            auto: AutoIndexConfig {
                flat_max_vectors: 5,
                ivf_max_vectors: 10,
                ..AutoIndexConfig::default()
            },
        },
        ..CollectionConfig::default()
    };
    let mut collection = Collection::open_with_options(path, config.into()).unwrap();
    let vector = |i: usize| {
        let angle = i as f32 * 0.4;
        vec![angle.cos(), angle.sin(), 0.1 * i as f32]
    };

    let mut families = Vec::new();
    for i in 0..12 {
        collection
            .insert(Document::new(vector(i), format!("doc{i}")))
            .unwrap();
        families.push(collection.vector_index().index_type());
    }
    assert_eq!(families[3], IndexType::Flat);
    assert_eq!(
        families[4],
        IndexType::Ivf,
        "the fifth vector reaches flat_max_vectors"
    );
    assert_eq!(
        families[9],
        IndexType::Hnsw,
        "the tenth vector reaches ivf_max_vectors"
    );

    let hits = collection
        .search(&vector(7), 1, Metric::Cosine, SearchParams::default())
        .unwrap();
    assert_eq!(hits[0].document.text, "doc7");
}

// Pages come in id order, so walking them visits every document exactly once.
#[test]
fn pages_walk_every_document_once_in_id_order() {
    let path = concat!(env!("CARGO_TARGET_TMPDIR"), "/test_pages.db");
    for suffix in [
        "",
        ".offsets.db",
        ".wal.db",
        ".vecindex.db",
        ".manifest.db",
        ".wal.meta",
    ] {
        let _ = fs::remove_file(format!("{path}{suffix}"));
    }
    let mut collection = Collection::open(path).unwrap();
    let mut ids: Vec<uuid::Uuid> = (0..23)
        .map(|i| {
            collection
                .insert(Document::new(vec![i as f32, 1.0], format!("doc{i}")))
                .unwrap()
        })
        .collect();
    ids.sort_unstable();

    let mut walked = Vec::new();
    let mut offset = 0;
    loop {
        let page = collection.page(offset, 5).unwrap();
        if page.is_empty() {
            break;
        }
        offset += page.len();
        walked.extend(page.into_iter().map(|document| document.id));
    }
    assert_eq!(walked, ids);
}

fn fresh_path(name: &str) -> String {
    let path = format!("{}/{name}", env!("CARGO_TARGET_TMPDIR"));
    for suffix in [
        "",
        ".offsets.db",
        ".wal.db",
        ".vecindex.db",
        ".manifest.db",
        ".wal.meta",
    ] {
        let _ = fs::remove_file(format!("{path}{suffix}"));
    }
    path
}

// A write refused for its width or a limit leaves nothing behind: not stored, not counted, and not
// in the log to be replayed on the next open.
#[test]
fn a_refused_write_leaves_nothing_behind() {
    let path = fresh_path("test_refused_write.db");
    {
        let mut collection = Collection::open(&path).unwrap();
        collection
            .insert(Document::new(vec![1.0, 0.0], "two wide".to_string()))
            .unwrap();

        let wrong = Document::new(vec![1.0, 0.0, 0.0], "three wide".to_string());
        let wrong_id = wrong.id;
        assert!(collection.insert(wrong).is_err());
        assert!(collection.get(&wrong_id).unwrap().is_none());

        let batch = vec![
            Document::new(vec![0.0, 1.0], "fits".to_string()),
            Document::new(vec![0.0, 1.0, 2.0], "does not".to_string()),
        ];
        let fits = batch[0].id;
        assert!(collection.insert_batch(batch).is_err());
        assert!(
            collection.get(&fits).unwrap().is_none(),
            "a refused batch stores none of it"
        );
        assert_eq!(collection.count(), 1);
    }
    let reopened = Collection::open(&path).unwrap();
    assert_eq!(reopened.count(), 1, "nothing refused was replayed");
}

// Replacing a stored document at the vector limit does not add a vector, so it is allowed.
#[test]
fn replacing_a_document_at_the_vector_limit_is_allowed() {
    use piramid_core::config::CollectionConfig;

    let path = fresh_path("test_replace_at_limit.db");
    let mut config = CollectionConfig::default();
    config.limits.max_vectors = Some(1);
    let mut collection = Collection::open_with_options(&path, config.into()).unwrap();
    let mut document = Document::new(vec![1.0, 0.0], "first".to_string());
    let id = collection.insert(document.clone()).unwrap();
    assert!(collection
        .insert(Document::new(vec![0.0, 1.0], "second".to_string()))
        .is_err());

    document.text = "replaced".to_string();
    collection.upsert(document).unwrap();
    assert!(collection.update_vector(&id, vec![0.5, 0.5]).unwrap());
    assert_eq!(collection.get(&id).unwrap().unwrap().text, "replaced");
    assert_eq!(collection.count(), 1);
}

#[test]
fn inserting_an_id_already_stored_is_refused() {
    let path = fresh_path("test_insert_existing_id.db");
    let mut collection = Collection::open(&path).unwrap();
    let document = Document::new(vec![1.0, 0.0], "first".to_string());
    collection.insert(document.clone()).unwrap();

    let error = collection.insert(document.clone()).unwrap_err();
    assert!(error.to_string().contains("already exists"), "{error}");

    let fresh = Document::new(vec![0.0, 1.0], "fresh".to_string());
    let fresh_id = fresh.id;
    let error = collection
        .insert_batch(vec![fresh.clone(), document])
        .unwrap_err();
    assert!(error.to_string().contains("already exists"), "{error}");

    let error = collection
        .insert_batch(vec![fresh.clone(), fresh])
        .unwrap_err();
    assert!(error.to_string().contains("more than once"), "{error}");

    assert!(collection.get(&fresh_id).unwrap().is_none());
    assert_eq!(collection.count(), 1);
    drop(collection);
    assert_eq!(Collection::open(&path).unwrap().count(), 1);
}

#[test]
fn offsets_without_a_manifest_are_refused_at_open() {
    let path = fresh_path("test_missing_manifest.db");
    {
        let mut collection = Collection::open(&path).unwrap();
        collection
            .insert(Document::new(vec![1.0, 0.0], "one".to_string()))
            .unwrap();
        collection.checkpoint().unwrap();
    }
    fs::remove_file(format!("{path}.manifest.db")).unwrap();

    let error = Collection::open(&path).err().unwrap();
    assert!(error.to_string().contains("no manifest"), "{error}");
}

// A missing index sidecar is rebuilt from the stored documents even when the WAL has entries to
// replay.
#[test]
fn a_missing_index_sidecar_is_rebuilt_beside_a_pending_wal() {
    let path = fresh_path("test_missing_vecindex_with_wal.db");
    let stored;
    {
        let mut collection = Collection::open(&path).unwrap();
        stored = collection
            .insert(Document::new(vec![1.0, 0.0], "checkpointed".to_string()))
            .unwrap();
        collection.checkpoint().unwrap();
        collection
            .insert(Document::new(vec![0.0, 1.0], "logged".to_string()))
            .unwrap();
        collection.flush().unwrap();
    }
    fs::remove_file(format!("{path}.vecindex.db")).unwrap();

    let collection = Collection::open(&path).unwrap();
    assert_eq!(collection.count(), 2);
    assert_eq!(collection.vector_index().stats().total_vectors, 2);
    let hits = collection
        .search(&[1.0, 0.0], 2, Metric::Cosine, SearchParams::default())
        .unwrap();
    assert!(hits.iter().any(|hit| hit.document.id == stored));
}

// An index sidecar built with another family or other parameters than the configuration is rebuilt
// at open.
#[test]
fn an_index_built_with_other_settings_is_rebuilt_at_open() {
    use piramid_core::config::{HnswConfig, IndexConfig};

    let path = fresh_path("test_index_config_changed.db");
    {
        let mut collection = Collection::open(&path).unwrap();
        for i in 0..8 {
            let angle = i as f32;
            collection
                .insert(Document::new(
                    vec![angle.cos(), angle.sin()],
                    format!("{i}"),
                ))
                .unwrap();
        }
        collection.checkpoint().unwrap();
    }

    let hnsw = |m: usize| CollectionConfig {
        index: IndexConfig::Hnsw {
            params: HnswConfig::from_m(m, 100, 100),
        },
        ..CollectionConfig::default()
    };

    let collection = Collection::open_with_options(&path, hnsw(8).into()).unwrap();
    assert_eq!(
        collection.vector_index().build_config(),
        IndexConfig::Hnsw {
            params: HnswConfig::from_m(8, 100, 100)
        }
    );
    assert_eq!(collection.vector_index().stats().total_vectors, 8);
    drop(collection);

    let collection = Collection::open_with_options(&path, hnsw(4).into()).unwrap();
    assert_eq!(
        collection.vector_index().build_config(),
        IndexConfig::Hnsw {
            params: HnswConfig::from_m(4, 100, 100)
        }
    );
    drop(collection);

    // The rebuilt index was saved, so the same configuration opens it unchanged.
    let collection = Collection::open_with_options(&path, hnsw(4).into()).unwrap();
    assert_eq!(collection.vector_index().stats().total_vectors, 8);
}

/// A manager over a fresh directory holding empty files with the given names.
fn manager_over_files(dir_name: &str, file_names: &[&str]) -> piramid_database::CollectionManager {
    let dir = std::path::PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(dir_name);
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    for name in file_names {
        fs::write(dir.join(name), b"").unwrap();
    }
    piramid_database::CollectionManager::new(
        dir.to_string_lossy().into_owned(),
        std::sync::Arc::new(parking_lot::RwLock::new(
            piramid_core::config::Config::default(),
        )),
    )
}

#[test]
fn sidecars_are_not_collections() {
    let manager = manager_over_files(
        "manager-sidecars",
        &[
            "docs.db",
            "docs.db.wal.db",
            "docs.db.offsets.db",
            "docs.db.vecindex.db",
            "docs.db.manifest.db",
        ],
    );
    assert_eq!(manager.discover_on_disk().unwrap(), ["docs"]);
}

#[test]
fn unrelated_files_are_ignored() {
    let manager = manager_over_files(
        "manager-unrelated",
        &["notes.txt", ".db", "docs.db.wal.meta", "docs.db.compact"],
    );
    assert!(manager.discover_on_disk().unwrap().is_empty());
}

#[test]
fn an_unreadable_data_directory_is_an_error_not_an_empty_listing() {
    let missing = std::path::PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
        .join("piramid-missing-data-dir/does-not-exist");
    let manager = piramid_database::CollectionManager::new(
        missing.to_string_lossy().into_owned(),
        std::sync::Arc::new(parking_lot::RwLock::new(
            piramid_core::config::Config::default(),
        )),
    );
    assert!(manager.discover_on_disk().is_err());
}
