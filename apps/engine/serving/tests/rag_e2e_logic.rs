//! The pure parts of the end-to-end benchmark in benches/rag_e2e, run as ordinary tests.

#[allow(dead_code, reason = "the benchmark uses items these tests do not")]
#[path = "../benches/rag_e2e/dataset.rs"]
mod dataset;
#[allow(dead_code, reason = "the benchmark uses items these tests do not")]
#[path = "../benches/rag_e2e/plan.rs"]
mod plan;
#[allow(dead_code, reason = "the benchmark uses items these tests do not")]
#[path = "../benches/rag_e2e/report.rs"]
mod report;
#[allow(dead_code, reason = "the benchmark uses items these tests do not")]
#[path = "../benches/rag_e2e/scoring.rs"]
mod scoring;

#[allow(
    clippy::unwrap_used,
    reason = "a failed assertion is the point of a test"
)]
mod dataset_tests {
    use super::dataset::*;

    const LINE: &str = r#"{"id":"q1","question":"Where is Paris?","answers":["France"],"passages":[{"id":"p1","text":"Paris is in France.","gold":true},{"id":"p2","text":"Lyon.","gold":false}]}"#;

    #[test]
    fn a_line_parses_into_a_question_with_its_gold_ids() {
        let question = parse_line(LINE, 1).unwrap();
        assert_eq!(question.id, "q1");
        assert_eq!(question.answers, vec!["France".to_string()]);
        assert_eq!(question.passages.len(), 2);
        assert_eq!(question.gold_ids(), vec!["p1"]);
    }

    #[test]
    fn a_line_missing_a_field_or_holding_an_unknown_one_is_refused() {
        let missing = r#"{"id":"q1","question":"q","answers":["a"]}"#;
        assert!(parse_line(missing, 3)
            .unwrap_err()
            .starts_with("dataset line 3:"));
        let unknown = r#"{"id":"q1","question":"q","answers":["a"],"passages":[],"extra":1}"#;
        assert!(parse_line(unknown, 1).is_err());
    }

    #[test]
    fn a_line_without_answers_or_passages_is_refused() {
        let no_answers = r#"{"id":"q","question":"q","answers":[" "],"passages":[{"id":"p","text":"t","gold":true}]}"#;
        assert!(parse_line(no_answers, 1).unwrap_err().contains("answers"));
        let no_passages = r#"{"id":"q","question":"q","answers":["a"],"passages":[]}"#;
        assert!(parse_line(no_passages, 1).unwrap_err().contains("passages"));
    }

    #[test]
    fn a_file_skips_blank_lines_honours_the_limit_and_refuses_repeated_ids() {
        let second = LINE.replace("\"q1\"", "\"q2\"");
        let contents = format!("{LINE}\n\n{second}\n");
        assert_eq!(parse(&contents, None).unwrap().len(), 2);
        assert_eq!(parse(&contents, Some(1)).unwrap().len(), 1);
        let repeated = format!("{LINE}\n{LINE}\n");
        assert!(parse(&repeated, None)
            .unwrap_err()
            .contains("more than once"));
        assert!(parse("\n\n", None).is_err());
    }

    #[test]
    fn passages_shared_between_questions_are_kept_once() {
        let first = parse_line(LINE, 1).unwrap();
        let mut second = first.clone();
        second.id = "q2".to_string();
        second.passages.push(Passage {
            id: "p3".to_string(),
            text: "Nice.".to_string(),
            gold: true,
        });
        let questions = [first, second];
        let ids: Vec<&str> = unique_passages(&questions)
            .iter()
            .map(|passage| passage.id.as_str())
            .collect();
        assert_eq!(ids, vec!["p1", "p2", "p3"]);
    }
}

#[allow(
    clippy::unwrap_used,
    reason = "a failed assertion is the point of a test"
)]
mod plan_tests {
    use std::collections::HashMap;
    use std::path::PathBuf;

    use super::plan::*;

    const EMBEDDING: &str =
        r#"{"provider":"ollama","model":"nomic-embed-text","base_url":"http://localhost:11434"}"#;

    fn plan(pairs: &[(&str, &str)]) -> Result<Plan, String> {
        let mut vars: HashMap<String, String> = [
            ("PIRAMID_BENCH_MODEL", "/models/qwen"),
            ("PIRAMID_BENCH_DATASET", "/data/q.jsonl"),
            ("PIRAMID_BENCH_EMBEDDING", EMBEDDING),
        ]
        .into_iter()
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect();
        for (name, value) in pairs {
            vars.insert((*name).to_string(), (*value).to_string());
        }
        Plan::from_lookup(
            |name| Ok(vars.get(name).cloned()),
            PathBuf::from("target/out.json"),
        )
    }

    #[test]
    fn defaults_fill_what_the_environment_leaves_unset() {
        let plan = plan(&[]).unwrap();
        assert_eq!(plan.device, "cpu");
        assert_eq!(plan.k, DEFAULT_K);
        assert_eq!(plan.arms, vec![Arm::ClosedBook, Arm::BeforePrefillHost]);
        assert_eq!(plan.out, PathBuf::from("target/out.json"));
        assert_eq!(plan.questions, None);
        assert_eq!(plan.embedding.provider.as_str(), "ollama");
        assert!(!plan.embedding.cache.enabled);
    }

    #[test]
    fn a_missing_required_variable_is_named() {
        let vars: HashMap<&str, &str> = HashMap::new();
        let error = Plan::from_lookup(
            |name| Ok(vars.get(name).map(|v| (*v).to_string())),
            PathBuf::new(),
        )
        .unwrap_err();
        assert!(error.contains("PIRAMID_BENCH_EMBEDDING"));
    }

    #[test]
    fn arms_parse_in_order_and_refuse_unknown_or_repeated_names() {
        let plan = plan(&[("PIRAMID_BENCH_ARMS", "before-prefill-host, closed-book")]).unwrap();
        assert_eq!(plan.arms, vec![Arm::BeforePrefillHost, Arm::ClosedBook]);
        assert!(plan_error(&[("PIRAMID_BENCH_ARMS", "open-book")]).contains("unknown arm"));
        assert!(
            plan_error(&[("PIRAMID_BENCH_ARMS", "closed-book,closed-book")])
                .contains("more than once")
        );
        assert!(plan_error(&[("PIRAMID_BENCH_ARMS", ",")]).contains("no arm"));
        for arm in Arm::ALL {
            assert_eq!(Arm::parse(arm.as_str()).unwrap(), arm);
        }
    }

    fn plan_error(pairs: &[(&str, &str)]) -> String {
        plan(pairs).unwrap_err()
    }

    #[test]
    fn numbers_are_checked() {
        assert!(plan_error(&[("PIRAMID_BENCH_K", "0")]).contains(">= 1"));
        assert!(plan_error(&[("PIRAMID_BENCH_K", "five")]).contains("PIRAMID_BENCH_K"));
        assert!(plan_error(&[("PIRAMID_BENCH_QUESTIONS", "0")]).contains(">= 1"));
        assert_eq!(
            plan(&[("PIRAMID_BENCH_QUESTIONS", "200")])
                .unwrap()
                .questions,
            Some(200)
        );
    }

    #[test]
    fn the_device_arm_is_refused_without_the_gpu_feature() {
        let plan = plan(&[
            ("PIRAMID_BENCH_ARMS", "before-prefill-device"),
            ("PIRAMID_BENCH_DEVICE", "cuda:0"),
        ])
        .unwrap();
        let error = plan.check_arms(false).unwrap_err();
        assert!(error.contains("gpu-cuda"));
        assert!(plan.check_arms(true).is_ok());
    }

    #[test]
    fn the_device_arm_is_refused_without_a_cuda_device() {
        let plan = plan(&[("PIRAMID_BENCH_ARMS", "before-prefill-device")]).unwrap();
        let error = plan.check_arms(true).unwrap_err();
        assert!(
            error.contains("PIRAMID_BENCH_DEVICE set to cuda:N"),
            "{error}"
        );
    }

    #[test]
    fn a_variable_the_lookup_cannot_read_is_an_error() {
        let error = Plan::from_lookup(
            |name| {
                if name == "PIRAMID_BENCH_DEVICE" {
                    Err(format!("{name} is not valid UTF-8"))
                } else {
                    Ok(match name {
                        "PIRAMID_BENCH_MODEL" => Some("/models/qwen".to_string()),
                        "PIRAMID_BENCH_DATASET" => Some("/data/q.jsonl".to_string()),
                        "PIRAMID_BENCH_EMBEDDING" => Some(EMBEDDING.to_string()),
                        _ => None,
                    })
                }
            },
            PathBuf::new(),
        )
        .unwrap_err();
        assert!(
            error.contains("PIRAMID_BENCH_DEVICE is not valid UTF-8"),
            "{error}"
        );
    }

    #[test]
    fn the_http_arm_needs_a_search_text_url() {
        let without = plan(&[("PIRAMID_BENCH_ARMS", "before-prefill-http")]).unwrap();
        assert!(without
            .check_arms(true)
            .unwrap_err()
            .contains("PIRAMID_BENCH_SEARCH_URL"));
        let with = plan(&[
            ("PIRAMID_BENCH_ARMS", "before-prefill-http"),
            (
                "PIRAMID_BENCH_SEARCH_URL",
                "http://127.0.0.1:6333/api/collections/bench/search/text",
            ),
        ])
        .unwrap();
        assert!(with.check_arms(false).is_ok());
    }

    #[test]
    fn the_collection_url_is_the_search_url_without_its_suffix() {
        assert_eq!(
            collection_url("http://h:6333/api/collections/bench/search/text").unwrap(),
            "http://h:6333/api/collections/bench"
        );
        assert!(collection_url("http://h:6333/api/collections/bench/search").is_err());
        assert!(collection_url("http://h:6333/search/text").is_err());
    }
}

mod report_tests {
    use super::report::*;

    fn record(arm: &'static str, ttft: f64, recall: Option<bool>, matched: bool) -> Record {
        Record {
            question: "q".to_string(),
            arm,
            embed_ms: recall.map(|_| 2.0),
            search_ms: recall.map(|_| 1.0),
            fetch_ms: recall.map(|_| 0.5),
            retrieval_ms: recall.map(|_| 3.5),
            prefill_ms: Some(ttft - 1.0),
            ttft_ms: Some(ttft),
            decode_tokens_per_sec: Some(ttft / 10.0),
            prompt_tokens: 100,
            completion_tokens: 8,
            recall,
            exact_match: matched,
            output: "out".to_string(),
        }
    }

    #[test]
    fn a_summary_covers_only_its_own_arm() {
        let records = [
            record("closed-book", 10.0, None, false),
            record("closed-book", 30.0, None, true),
            record("before-prefill-host", 20.0, Some(true), true),
            record("before-prefill-host", 40.0, Some(false), true),
            record("before-prefill-host", 60.0, Some(true), false),
        ];
        let closed = summarize("closed-book", &records);
        assert_eq!(closed.questions, 2);
        assert_eq!(closed.embed_ms, None);
        assert_eq!(closed.recall_at_k, None);
        assert_eq!(closed.exact_match, Some(0.5));
        assert_eq!(closed.ttft_ms.map(|s| s.p50), Some(20.0));

        let host = summarize("before-prefill-host", &records);
        assert_eq!(host.questions, 3);
        assert_eq!(host.ttft_ms.map(|s| s.p50), Some(40.0));
        assert_eq!(host.decode_tokens_per_sec, Some(4.0));
        assert_eq!(host.completion_tokens, Some(8.0));
        let recall = host.recall_at_k.unwrap_or_default();
        assert!((recall - 2.0 / 3.0).abs() < 1e-9);

        let empty = summarize("before-prefill-device", &records);
        assert_eq!(empty.questions, 0);
        assert_eq!(empty.exact_match, None);
        assert_eq!(empty.ttft_ms, None);
    }

    #[test]
    fn the_table_has_a_header_a_rule_and_one_row_per_arm() {
        let records = [
            record("closed-book", 10.0, None, true),
            record("before-prefill-host", 20.0, Some(true), false),
        ];
        let summaries = [
            summarize("closed-book", &records),
            summarize("before-prefill-host", &records),
        ];
        let table = markdown_table(&summaries, 5);
        let lines: Vec<&str> = table.lines().collect();
        assert_eq!(lines.len(), 4);
        assert!(lines[0].contains("recall@5"));
        assert!(lines[1].starts_with("|---|"));
        assert_eq!(
            lines[2],
            "| closed-book | 1 | - | - | - | - | 9.0 / 9.0 | 10.0 / 10.0 | 1.0 | - | 100.0% |"
        );
        assert_eq!(
            lines[3],
            "| before-prefill-host | 1 | 2.0 / 2.0 | 1.0 / 1.0 | 0.5 / 0.5 | 3.5 / 3.5 | 19.0 / 19.0 | 20.0 / 20.0 | 2.0 | 100.0% | 0.0% |"
        );
        let columns = |line: &str| line.matches('|').count();
        assert!(lines.iter().all(|line| columns(line) == columns(lines[0])));
    }
}

#[allow(
    clippy::unwrap_used,
    reason = "a failed assertion is the point of a test"
)]
mod scoring_tests {
    use std::time::Duration;

    use super::scoring::*;

    #[test]
    fn normalization_drops_case_punctuation_articles_and_extra_space() {
        assert_eq!(
            normalize_answer("  The Eiffel-Tower, in  Paris! "),
            "eiffeltower in paris"
        );
        assert_eq!(normalize_answer("U.S.A."), "usa");
        assert_eq!(normalize_answer("An apple a day"), "apple day");
        assert_eq!(normalize_answer("..."), "");
    }

    #[test]
    fn a_match_needs_the_whole_answer_as_words_of_the_output() {
        let answers = vec!["The United States".to_string(), "USA".to_string()];
        assert!(exact_match("It is in the United States.", &answers));
        assert!(exact_match("usa", &answers));
        assert!(!exact_match("It is in the United Kingdom.", &answers));
        assert!(!exact_match("no", &["not".to_string()]));
        assert!(!exact_match("not sure", &["no".to_string()]));
        assert!(!exact_match("anything", &["the".to_string()]));
    }

    #[test]
    fn recall_is_any_gold_id_among_the_retrieved() {
        let retrieved = vec!["p2".to_string(), "p7".to_string()];
        assert_eq!(recall_at_k(&retrieved, &["p7", "p9"]), Some(true));
        assert_eq!(recall_at_k(&retrieved, &["p9"]), Some(false));
        assert_eq!(recall_at_k(&retrieved, &[]), None);
        assert_eq!(recall_at_k(&[], &["p1"]), Some(false));
    }

    #[test]
    fn the_decode_rate_counts_tokens_after_the_first() {
        let rate = decode_tokens_per_second(
            11,
            Some(Duration::from_millis(500)),
            Duration::from_millis(1500),
        )
        .unwrap();
        assert!((rate - 10.0).abs() < 1e-9);
        assert_eq!(
            decode_tokens_per_second(1, Some(Duration::ZERO), Duration::from_secs(1)),
            None
        );
        assert_eq!(
            decode_tokens_per_second(5, None, Duration::from_secs(1)),
            None
        );
        assert_eq!(
            decode_tokens_per_second(5, Some(Duration::from_secs(1)), Duration::from_secs(1)),
            None
        );
    }

    #[test]
    fn percentiles_interpolate_between_ranks() {
        let values = [4.0, 1.0, 3.0, 2.0, 5.0];
        assert_eq!(percentile(&values, 0.5), Some(3.0));
        assert_eq!(percentile(&values, 0.0), Some(1.0));
        assert_eq!(percentile(&values, 1.0), Some(5.0));
        let p95 = percentile(&values, 0.95).unwrap();
        assert!((p95 - 4.8).abs() < 1e-9);
        assert_eq!(percentile(&[7.0], 0.95), Some(7.0));
        assert_eq!(percentile(&[], 0.5), None);
        assert_eq!(percentile(&values, 1.5), None);
    }

    #[test]
    fn the_mean_of_nothing_is_none() {
        assert_eq!(mean(&[1.0, 2.0, 6.0]), Some(3.0));
        assert_eq!(mean(&[]), None);
    }
}
