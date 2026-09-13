#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "assertions in tests"
)]
//! Collections: storage, persistence, WAL, checkpoints, resident state, metric and compaction.

use std::fs;
use {
    piramid_core::config::CollectionConfig,
    piramid_core::config::MemoryConfig,
    piramid_core::metadata::metadata,
    piramid_core::Document,
    piramid_database::search::SearchParams,
    piramid_database::storage::SidecarManager,
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

    storage.checkpoint().unwrap();

    assert!(fs::metadata(format!("{test_path}.offsets.db")).is_ok());
    assert!(fs::metadata(format!("{test_path}.vecindex.db")).is_err());

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
fn compaction_reclaims_deleted_records() {
    let test_path = fresh_path("test_record_store_compact.db");

    let mut storage = Collection::open(&test_path).unwrap();
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
    remove_collection_files(&test_path);
}

// A collection with sync_on_write enabled accepts writes and replays them on reopen.
#[test]
fn writes_replay_with_sync_on_write_enabled() {
    use piramid_core::config::CollectionConfig;

    let path = concat!(env!("CARGO_TARGET_TMPDIR"), "/test_wal_sync.db");
    for suffix in ["", ".offsets.db", ".wal.db", ".manifest.db", ".wal.meta"] {
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
    for suffix in ["", ".offsets.db", ".wal.db", ".manifest.db", ".wal.meta"] {
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

    // A delete marks its row as a hole until an insert reuses it.
    storage.delete(&ids[1]).unwrap();
    let slab = storage.vector_reader().as_slab().unwrap();
    assert_eq!(slab.live.iter().filter(|live| !**live).count(), 1);

    storage
        .insert(Document::new(vec![9.0, 9.0, 9.0], "refill".to_string()))
        .unwrap();
    let slab = storage
        .vector_reader()
        .as_slab()
        .expect("a collection with vectors hands over a slab");
    assert_eq!(slab.data.len(), 4 * 3);
    assert!(slab.live.iter().all(|live| *live), "the hole was reused");

    drop(storage);
    cleanup_test_files(&files);
}

// The interval trigger counts from the open, so it fires before any other trigger runs.
#[test]
fn the_checkpoint_interval_runs_from_the_open() {
    use piramid_core::config::CollectionConfig;

    let path = concat!(env!("CARGO_TARGET_TMPDIR"), "/test_wal_interval.db");
    for suffix in ["", ".offsets.db", ".wal.db", ".manifest.db", ".wal.meta"] {
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

// Pages come in id order, so walking them visits every document exactly once.
#[test]
fn pages_walk_every_document_once_in_id_order() {
    let path = concat!(env!("CARGO_TARGET_TMPDIR"), "/test_pages.db");
    for suffix in ["", ".offsets.db", ".wal.db", ".manifest.db", ".wal.meta"] {
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
    ensure_test_dir();
    let path = format!("{}/{name}", env!("CARGO_TARGET_TMPDIR"));
    remove_collection_files(&path);
    path
}

/// Remove the data file of the collection at path and every sidecar beside it.
fn remove_collection_files(path: &str) {
    let _ = fs::remove_file(path);
    let _ = fs::remove_dir_all(path);
    for sidecar in SidecarManager::at(path).all_paths() {
        let _ = fs::remove_file(&sidecar);
        let _ = fs::remove_file(format!("{sidecar}.tmp"));
    }
}

// A refused write leaves nothing behind: not stored, not counted, not in the log.
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

/// A fresh directory holding empty files with the given names.
fn dir_of_files(dir_name: &str, file_names: &[&str]) -> String {
    let dir = std::path::PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(dir_name);
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    for name in file_names {
        fs::write(dir.join(name), b"").unwrap();
    }
    dir.to_string_lossy().into_owned()
}

#[test]
fn sidecars_are_not_collections() {
    let dir = dir_of_files(
        "manager-sidecars",
        &[
            "docs.db",
            "docs.db.wal.db",
            "docs.db.offsets.db",
            "docs.db.vecindex.db",
            "docs.db.compact.offsets",
            "docs.db.compact.commit",
            "docs.db.manifest.db",
        ],
    );
    assert_eq!(piramid_database::collection_names(&dir).unwrap(), ["docs"]);
}

#[test]
fn unrelated_files_are_ignored() {
    let dir = dir_of_files(
        "manager-unrelated",
        &["notes.txt", ".db", "docs.db.wal.meta", "docs.db.compact"],
    );
    assert!(piramid_database::collection_names(&dir).unwrap().is_empty());
}

#[test]
fn an_unreadable_data_directory_is_an_error_not_an_empty_listing() {
    let missing = std::path::PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
        .join("piramid-missing-data-dir/does-not-exist");
    assert!(piramid_database::collection_names(&missing.to_string_lossy()).is_err());
}

// Every live document has resident metadata after every mutation, reopen and WAL replay.
#[test]
fn metadata_is_resident_for_every_live_document() {
    let path = fresh_path("test_resident_metadata.db");
    let kind = |value: &str| metadata([("kind", value.into())]);
    let (first, second, third);
    {
        let mut collection = Collection::open(&path).unwrap();
        first = collection
            .insert(Document::with_metadata(
                vec![1.0, 0.0],
                "first".to_string(),
                kind("inserted"),
            ))
            .unwrap();
        let batch = collection
            .insert_batch(vec![
                Document::with_metadata(vec![0.0, 1.0], "second".to_string(), kind("batched")),
                Document::with_metadata(vec![1.0, 1.0], "third".to_string(), kind("batched")),
            ])
            .unwrap();
        (second, third) = (batch[0], batch[1]);
        collection.checkpoint().unwrap();

        let mut replaced = collection.get(&second).unwrap().unwrap();
        replaced.metadata = kind("upserted");
        collection.upsert(replaced).unwrap();
        collection.update_metadata(&first, kind("updated")).unwrap();
        collection.delete(&third).unwrap();

        let view = collection.metadata_view();
        assert_eq!(view.len(), 2);
        assert_eq!(view[&first], kind("updated"));
        assert_eq!(view[&second], kind("upserted"));
        assert!(!view.contains_key(&third));
        collection.flush().unwrap();
    }

    // The upsert, update and delete after the checkpoint replay from the WAL.
    let reopened = Collection::open(&path).unwrap();
    let view = reopened.metadata_view();
    assert_eq!(view.len(), 2);
    assert_eq!(view[&first], kind("updated"));
    assert_eq!(view[&second], kind("upserted"));
    assert_eq!(reopened.vector_reader().len(), 2);
}

// A reopen keeps the metric stored at creation even under a different configured metric.
#[test]
fn the_metric_is_stored_in_the_manifest_and_survives_a_config_change() {
    let path = fresh_path("test_metric_persisted.db");
    let with_metric = |metric| {
        let mut config = CollectionConfig::default();
        config.search.metric = metric;
        CollectionOpenOptions { config }
    };
    {
        let collection =
            Collection::open_with_options(&path, with_metric(Metric::DotProduct)).unwrap();
        assert_eq!(collection.metric(), Metric::DotProduct);
    }
    let stored = SidecarManager::at(&path).load_manifest().unwrap().unwrap();
    assert_eq!(stored.metric, Metric::DotProduct);
    assert_eq!(stored.schema_version, 2);

    let mut reopened = Collection::open_with_options(&path, with_metric(Metric::Cosine)).unwrap();
    assert_eq!(reopened.metric(), Metric::DotProduct);

    let mut next = reopened.config().clone();
    next.search.metric = Metric::Euclidean;
    next.search.parallel = !next.search.parallel;
    reopened.apply_live_settings(&next).unwrap();
    assert_eq!(reopened.metric(), Metric::DotProduct);
    assert_eq!(reopened.config().search.parallel, next.search.parallel);
}

#[test]
fn a_setting_that_needs_a_reopen_is_refused_live() {
    let path = fresh_path("test_reopen_setting.db");
    let mut collection = Collection::open(&path).unwrap();
    let mut next = collection.config().clone();
    next.wal.enabled = !next.wal.enabled;

    assert_eq!(
        collection.setting_needing_reopen(&next),
        Some("runtime.wal.enabled")
    );
    let error = collection.apply_live_settings(&next).unwrap_err();
    assert_eq!(error.kind(), piramid_core::error::ErrorKind::BadRequest);
    assert!(error.to_string().contains("runtime.wal.enabled"), "{error}");
}

// A schema 1 manifest is refused with an error naming the collection, nothing else touched.
#[test]
fn a_schema_1_manifest_is_refused() {
    use piramid_core::error::{PiramidError, StorageError};

    let path = fresh_path("test_legacy_manifest.db");
    let mut legacy = Vec::new();
    legacy.extend_from_slice(&1u32.to_le_bytes());
    legacy.extend_from_slice(&6u64.to_le_bytes());
    legacy.extend_from_slice(b"legacy");
    legacy.extend_from_slice(&1_700_000_000u64.to_le_bytes());
    legacy.extend_from_slice(&1_700_000_500u64.to_le_bytes());
    legacy.push(1);
    legacy.extend_from_slice(&3u64.to_le_bytes());
    legacy.extend_from_slice(&1u64.to_le_bytes());
    fs::write(format!("{path}.manifest.db"), &legacy).unwrap();
    fs::write(format!("{path}.vecindex.db"), b"old index").unwrap();

    let error = Collection::open(&path).err().unwrap();
    assert!(
        matches!(
            &error,
            PiramidError::Storage(StorageError::LegacyManifest { collection }) if collection == "legacy"
        ),
        "{error}"
    );
    assert!(error.to_string().contains("Piramid 0.2"), "{error}");
    assert!(error.to_string().contains("re-ingested"), "{error}");
    assert_eq!(
        fs::read(format!("{path}.vecindex.db")).unwrap(),
        b"old index"
    );
    assert!(fs::metadata(&path).is_err(), "no data file is created");
}

/// The files of a collection that compaction reads and replaces, by suffix.
const COMPACTED_FILES: [&str; 5] = ["", ".offsets.db", ".manifest.db", ".wal.db", ".wal.meta"];

/// Copy every compacted file of the collection at from to the collection at to.
fn copy_collection(from: &str, to: &str) {
    for suffix in COMPACTED_FILES {
        let source = format!("{from}{suffix}");
        if fs::metadata(&source).is_ok() {
            fs::copy(&source, format!("{to}{suffix}")).unwrap();
        }
    }
}

/// Builds a checkpointed collection with deleted documents, plus a compacted copy of it.
fn collection_before_and_after_compaction(name: &str) -> (String, String, Vec<uuid::Uuid>) {
    let before = fresh_path(&format!("{name}.db"));
    let after = fresh_path(&format!("{name}_compacted.db"));
    let mut live = Vec::new();
    {
        let mut collection = Collection::open(&before).unwrap();
        for i in 0..12 {
            let id = collection
                .insert(Document::with_metadata(
                    vec![i as f32 + 1.0, 1.0],
                    format!("doc{i}"),
                    metadata([("i", i64::from(i).into())]),
                ))
                .unwrap();
            if i % 3 == 0 {
                collection.delete(&id).unwrap();
            } else {
                live.push(id);
            }
        }
        collection.checkpoint().unwrap();
    }
    copy_collection(&before, &after);
    {
        let mut collection = Collection::open(&after).unwrap();
        compact(&mut collection).unwrap();
    }
    (before, after, live)
}

/// Opens the collection at path and asserts it holds exactly the live documents, searchable.
fn assert_opens_with(path: &str, live: &[uuid::Uuid]) {
    let collection = Collection::open(path).unwrap();
    assert_eq!(collection.count(), live.len());
    for id in live {
        let document = collection.get(id).unwrap().unwrap();
        assert!(collection.metadata_view().contains_key(id));
        assert!(document.text.starts_with("doc"));
    }
    let hits = collection
        .search(
            &[4.0, 1.0],
            live.len(),
            Metric::Cosine,
            SearchParams::default(),
        )
        .unwrap();
    assert_eq!(hits.len(), live.len());
    let sidecars = SidecarManager::at(path);
    for leftover in [
        sidecars.compact_path(),
        sidecars.compact_offsets_path(),
        sidecars.compact_commit_path(),
    ] {
        assert!(fs::metadata(&leftover).is_err(), "{leftover} was left");
    }
}

// A crash while writing the compacted records leaves a partial file; open discards it.
#[test]
fn a_compaction_interrupted_while_writing_records_is_discarded() {
    let (before, _, live) = collection_before_and_after_compaction("crash_records");
    let sidecars = SidecarManager::at(&before);
    fs::write(sidecars.compact_path(), b"partial records").unwrap();

    assert_opens_with(&before, &live);
}

// A crash while writing the compacted offsets leaves a partial offsets file; open discards both.
#[test]
fn a_compaction_interrupted_while_writing_offsets_is_discarded() {
    let (before, after, live) = collection_before_and_after_compaction("crash_offsets");
    let sidecars = SidecarManager::at(&before);
    fs::copy(&after, sidecars.compact_path()).unwrap();
    fs::write(format!("{}.tmp", sidecars.compact_offsets_path()), b"half").unwrap();

    assert_opens_with(&before, &live);
}

// A crash before the commit marker exists discards the compaction.
#[test]
fn a_compaction_interrupted_before_its_commit_is_discarded() {
    let (before, after, live) = collection_before_and_after_compaction("crash_before_commit");
    let sidecars = SidecarManager::at(&before);
    fs::copy(&after, sidecars.compact_path()).unwrap();
    fs::copy(
        format!("{after}.offsets.db"),
        sidecars.compact_offsets_path(),
    )
    .unwrap();
    fs::write(format!("{}.tmp", sidecars.compact_commit_path()), b"").unwrap();

    assert_opens_with(&before, &live);
}

// A crash right after the commit marker is created finishes the compaction at open.
#[test]
fn a_committed_compaction_with_no_file_moved_is_finished() {
    let (before, after, live) = collection_before_and_after_compaction("crash_after_commit");
    let sidecars = SidecarManager::at(&before);
    fs::copy(&after, sidecars.compact_path()).unwrap();
    fs::copy(
        format!("{after}.offsets.db"),
        sidecars.compact_offsets_path(),
    )
    .unwrap();
    fs::write(sidecars.compact_commit_path(), b"").unwrap();

    assert_opens_with(&before, &live);
    assert_eq!(
        fs::read(&before).unwrap(),
        fs::read(&after).unwrap(),
        "the compacted record file was moved into place"
    );
}

// A crash after only the record file is moved finishes the compaction at open.
#[test]
fn a_committed_compaction_with_only_the_records_moved_is_finished() {
    let (before, after, live) = collection_before_and_after_compaction("crash_records_moved");
    let sidecars = SidecarManager::at(&before);
    fs::copy(&after, &before).unwrap();
    fs::copy(
        format!("{after}.offsets.db"),
        sidecars.compact_offsets_path(),
    )
    .unwrap();
    fs::write(sidecars.compact_commit_path(), b"").unwrap();

    assert_opens_with(&before, &live);
}

// A crash before the commit marker is removed opens the compacted collection and removes it.
#[test]
fn a_committed_compaction_with_both_files_moved_is_finished() {
    let (before, after, live) = collection_before_and_after_compaction("crash_both_moved");
    let sidecars = SidecarManager::at(&before);
    fs::copy(&after, &before).unwrap();
    fs::copy(format!("{after}.offsets.db"), sidecars.offsets_path()).unwrap();
    fs::write(sidecars.compact_commit_path(), b"").unwrap();

    assert_opens_with(&before, &live);
}

// A completed compaction leaves the collection usable, and writes after it replay on reopen.
#[test]
fn a_completed_compaction_reopens_and_accepts_writes() {
    let (_, after, live) = collection_before_and_after_compaction("compaction_complete");
    assert_opens_with(&after, &live);

    let added = {
        let mut collection = Collection::open(&after).unwrap();
        assert!(
            collection
                .vector_reader()
                .as_slab()
                .unwrap()
                .live
                .iter()
                .all(|live| *live),
            "compaction leaves no holes"
        );
        let added = collection
            .insert(Document::new(vec![0.5, 2.0], "doc added".to_string()))
            .unwrap();
        collection.flush().unwrap();
        added
    };
    let mut expected = live.clone();
    expected.push(added);
    assert_opens_with(&after, &expected);
}

// A compaction that cannot move its files into place refuses writes until the obstacle is gone.
#[test]
fn a_committed_compaction_that_cannot_finish_refuses_writes_until_reopen() {
    use piramid_core::error::{PiramidError, StorageError};

    let (before, _, live) = collection_before_and_after_compaction("unfinished_compaction");
    let mut collection = Collection::open(&before).unwrap();
    fs::remove_file(&before).unwrap();
    fs::create_dir(&before).unwrap();
    fs::write(format!("{before}/occupied"), b"").unwrap();

    let error = compact(&mut collection).unwrap_err();
    assert!(
        fs::metadata(SidecarManager::at(&before).compact_commit_path()).is_ok(),
        "the compaction was committed: {error}"
    );

    let refused = |error: PiramidError| {
        assert!(
            matches!(
                &error,
                PiramidError::Storage(StorageError::CompactionUnfinished { .. })
            ),
            "{error}"
        );
    };
    refused(
        collection
            .insert(Document::new(vec![3.0, 1.0], "doc after".to_string()))
            .unwrap_err(),
    );
    refused(collection.delete(&live[0]).unwrap_err());
    refused(collection.checkpoint().unwrap_err());
    refused(compact(&mut collection).unwrap_err());
    assert_eq!(collection.get(&live[0]).unwrap().unwrap().id, live[0]);
    drop(collection);

    fs::remove_dir_all(&before).unwrap();
    assert_opens_with(&before, &live);
    let added = {
        let mut collection = Collection::open(&before).unwrap();
        let added = collection
            .insert(Document::new(vec![0.5, 2.0], "doc added".to_string()))
            .unwrap();
        collection.checkpoint().unwrap();
        added
    };
    let mut expected = live.clone();
    expected.push(added);
    assert_opens_with(&before, &expected);
}
