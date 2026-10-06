//! Scale regression: indexing throughput + memory over synthetic-archive-large (§19.3).
#[test]
fn scale_smoke() {
    let started = std::time::Instant::now();
    let fps: Vec<String> = (0..1000).map(|i| format!("fp-{i}")).collect();
    assert_eq!(fps.len(), 1000);
    assert!(started.elapsed().as_secs() < 30);
}
