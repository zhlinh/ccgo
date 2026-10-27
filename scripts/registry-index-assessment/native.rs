//! Compiled in a temporary crate by native.py; imports production modules verbatim.
#![allow(dead_code, unused_imports)]
#[path = "__ROOT__/src/registry/mod.rs"]
mod registry;
#[path = "__ROOT__/src/version.rs"]
mod version;

use std::hint::black_box;
use std::time::Instant;

fn main() -> anyhow::Result<()> {
    let args: Vec<String> = std::env::args().collect();
    let wanted = &args[1];
    let repeats: usize = args[2].parse()?;
    let cache = registry::RegistryCache::new("bench", "file:///not-accessed");
    let candidates = vec![("bench".to_owned(), cache)];
    let mut samples = Vec::new();
    for _ in 0..repeats {
        let start = Instant::now();
        let resolved = registry::resolve_dep("stdcomm", wanted, &candidates)?.unwrap();
        assert_eq!(resolved.version_entry.version, *wanted);
        black_box(resolved);
        samples.push(start.elapsed().as_secs_f64() * 1000.0);
    }
    println!("{}", serde_json::to_string(&samples)?);
    Ok(())
}
