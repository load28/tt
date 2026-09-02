//! Input loading, external declarations, and bounded parallel mapping.

use super::*;

/// Declaration tables of the `.tt` modules a run imports, shared by every
/// job.
///
/// The same module is typically imported by many files, and each import
/// used to mean another disk read and another full parse of that module.
/// Here every module is read and parsed at most once per run, and modules
/// that are themselves inputs are served from the sources the run already
/// holds — no second read at all.
pub(super) struct ExternCache<'a> {
    /// The run's own input sources, keyed by path.
    inputs: HashMap<&'a Path, &'a str>,
    /// Exported declarations per imported path, filled on first use. An
    /// unreadable module caches as an empty table: module resolution is
    /// tsc's domain (`TS2307`), so its variants simply stay unknown.
    decls: Mutex<HashMap<PathBuf, Arc<Vec<ExternVariant>>>>,
}

impl<'a> ExternCache<'a> {
    pub(super) fn new(inputs: HashMap<&'a Path, &'a str>) -> Self {
        ExternCache {
            inputs,
            decls: Mutex::new(HashMap::new()),
        }
    }

    pub(super) fn exported_variants(&self, path: &Path) -> Arc<Vec<ExternVariant>> {
        // A poisoned lock means another job already panicked, and that
        // panic is the failure being reported — a second one here would
        // bury it. The map's contents are sound either way: it is only
        // ever inserted into, never left half-written (TASK-221).
        if let Some(hit) = self
            .decls
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .get(path)
        {
            return Arc::clone(hit);
        }
        // Parsed outside the lock: a slow miss must not stall other jobs.
        // Two jobs racing on the same module both parse it once; the first
        // insertion wins and both see the same table.
        let source_kind = ttc::SourceKind::from_path(path).unwrap_or_default();
        let decls = Arc::new(match self.inputs.get(path) {
            Some(source) => ttc::exported_variants_with_kind(source, source_kind),
            None => match fs::read_to_string(path) {
                Ok(source) => ttc::exported_variants_with_kind(&source, source_kind),
                Err(_) => Vec::new(),
            },
        });
        Arc::clone(
            self.decls
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner())
                .entry(path.to_path_buf())
                .or_insert(decls),
        )
    }
}

/// Collects variant declarations from the file's direct relative `.tt`
/// imports, so matches over imported variants get exhaustiveness-checked
/// (module graph phase 2). One hop, import declarations only — re-exports
/// bring nothing into scope. A specifier that cannot be read is skipped
/// silently: module resolution is tsc's domain (`TS2307`), and an unknown
/// variant simply stays unchecked, exactly as before.
pub(super) fn collect_extern_variants(
    file: &Path,
    imports: &[TtImport],
    cache: &ExternCache,
) -> Vec<ExternVariant> {
    let dir = file.parent().unwrap_or(Path::new("."));
    let mut externs: Vec<ExternVariant> = Vec::new();
    for import in imports {
        if matches!(import.names, TtImportNames::None) {
            continue;
        }
        let decls = cache.exported_variants(&dir.join(&import.specifier));
        let from = Some(import.specifier.clone());
        match &import.names {
            TtImportNames::Namespace(ns) => {
                externs.extend(decls.iter().map(|d| ExternVariant {
                    name: format!("{ns}.{}", d.name),
                    tags: d.tags.clone(),
                    from: from.clone(),
                }));
            }
            TtImportNames::Named(entries) => {
                for (name, alias) in entries {
                    if let Some(d) = decls.iter().find(|d| &d.name == name) {
                        externs.push(ExternVariant {
                            name: alias.clone().unwrap_or_else(|| name.clone()),
                            tags: d.tags.clone(),
                            from: from.clone(),
                        });
                    }
                }
            }
            // Skipped by the guard at the top of the loop: an import
            // that brings no names in has no declarations to collect.
            TtImportNames::None => unreachable!("a nameless import was skipped above"),
        }
    }
    externs
}

/// One input, read and scanned once for the whole run — or the diagnostic
/// its read failed with.
pub(super) struct Loaded {
    pub(super) source: String,
    pub(super) scan: ModuleScan,
}

/// Reads and scans every job's source, in parallel.
pub(super) fn load_jobs(jobs: &[Job], jobs_limit: Option<usize>) -> Vec<Result<Loaded, String>> {
    par_map(jobs, jobs_limit, |job| {
        let source =
            fs::read_to_string(&job.file).map_err(|e| format!("{}: {e}", job.file.display()))?;
        let scan = ttc::scan_module_with_kind(
            &source,
            ttc::SourceKind::from_path(&job.file).unwrap_or_default(),
        );
        Ok(Loaded { source, scan })
    })
}

/// How many worker threads a parallel phase should use: the `--jobs` value
/// when given, otherwise one per available core. Never more than there is
/// work for.
pub(super) fn worker_count(items: usize, jobs_limit: Option<usize>) -> usize {
    let want = jobs_limit.unwrap_or_else(|| {
        std::thread::available_parallelism()
            .map(|n| n.get())
            .unwrap_or(1)
    });
    want.clamp(1, items.max(1))
}

/// Maps `f` over `items` across worker threads, returning the results in
/// input order — so diagnostics and outputs stay byte-identical to a
/// sequential run whatever order the work actually finished in.
///
/// Compilation is per-file and shares nothing mutable, which is what makes
/// this the compiler's main lever on large trees; the ordered result is
/// what keeps the CLI deterministic.
pub(super) fn par_map<T, R>(
    items: &[T],
    jobs_limit: Option<usize>,
    f: impl Fn(&T) -> R + Sync,
) -> Vec<R>
where
    T: Sync,
    R: Send,
{
    let workers = worker_count(items.len(), jobs_limit);
    if workers <= 1 || items.len() <= 1 {
        return items.iter().map(f).collect();
    }
    let next = AtomicUsize::new(0);
    let f = &f;
    let batches: Vec<Vec<(usize, R)>> = std::thread::scope(|scope| {
        let handles: Vec<_> = (0..workers)
            .map(|_| {
                let next = &next;
                scope.spawn(move || {
                    let mut done: Vec<(usize, R)> = Vec::new();
                    loop {
                        let i = next.fetch_add(1, Ordering::Relaxed);
                        match items.get(i) {
                            Some(item) => done.push((i, f(item))),
                            None => return done,
                        }
                    }
                })
            })
            .collect();
        handles
            .into_iter()
            .map(|h| h.join().unwrap_or_else(|e| std::panic::resume_unwind(e)))
            .collect()
    });
    let mut slots: Vec<Option<R>> = (0..items.len()).map(|_| None).collect();
    for (i, r) in batches.into_iter().flatten() {
        slots[i] = Some(r);
    }
    slots
        .into_iter()
        // Each worker writes the slot of the index it was given, and the
        // indices are `0..items.len()` exactly once, so every slot is
        // filled before this runs.
        .map(|r| r.expect("each index produced its own result"))
        .collect()
}
