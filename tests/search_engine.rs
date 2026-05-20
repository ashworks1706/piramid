use piramid::{metadata, Collection, Document, Filter, Metric, SearchParams};
use std::fs;

fn cleanup(path: &str) {
    let sidecars = [
        format!("{}.index.db", path),
        format!("{}.wal.db", path),
        format!("{}.vecindex.db", path),
        format!("{}.metadata.db", path),
    ];
    for p in std::iter::once(path.to_string()).chain(sidecars) {
        let _ = fs::remove_file(p);
    }
}

#[test]
fn search_respects_filter() {
    let test_db = ".piramid/tests/test_search_filter.db";
    cleanup(test_db);

    {
        let mut storage = Collection::open(test_db).unwrap();

        let e1 = Document::with_metadata(
            vec![1.0, 0.0, 0.0],
            "rust doc".to_string(),
            metadata([("lang", "rust".into())]),
        );
        let e2 = Document::with_metadata(
            vec![0.9, 0.1, 0.0],
            "python doc".to_string(),
            metadata([("lang", "python".into())]),
        );

        storage.insert(e1).unwrap();
        storage.insert(e2).unwrap();

        let filter = Filter::new().eq("lang", "rust");
        let params = SearchParams {
            mode: storage.config().execution,
            filter: Some(&filter),
            filter_overfetch_override: None,
            search_config_override: None,
        };

        let results = piramid::search::engine::search_collection(
            &storage,
            &[1.0, 0.0, 0.0],
            5,
            Metric::Cosine,
            params,
        );
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].text, "rust doc");
    }

    cleanup(test_db);
}
