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
    let mut externs: Vec<ExternVariant> = Vec::new();
    for import in imports {
        let Some(module) = extern_module(file, import) else {
            continue;
        };
        let decls = cache.exported_variants(&module);
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
            TtImportNames::None => unreachable!("a nameless import was skipped above"),
        }
    }
    externs
}

pub(super) fn extern_module(file: &Path, import: &TtImport) -> Option<PathBuf> {
    if matches!(import.names, TtImportNames::None) {
        return None;
    }
    Some(file.parent().unwrap_or(Path::new(".")).join(import.path()))
}

pub(super) fn compile_reads(file: &Path) -> Vec<PathBuf> {
    let mut reads = vec![file.to_path_buf()];
    let Some(kind) = ttc::SourceKind::from_tt_path(file) else {
        return reads;
    };
    if let Ok(source) = fs::read_to_string(file) {
        reads.extend(
            ttc::tt_imports_with_kind(&source, kind)
                .iter()
                .filter_map(|import| extern_module(file, import)),
        );
    }
    reads
}

/// One input, read and scanned once for the whole run — or the diagnostic
/// its read failed with.
pub(super) struct Loaded {
    pub(super) source: String,
    pub(super) encoding: Option<Encoding>,
    pub(super) scan: ModuleScan,
}

/// Reads and scans every job's source, in parallel.
pub(super) fn load_jobs(jobs: &[Job], jobs_limit: Option<usize>) -> Vec<Result<Loaded, String>> {
    par_map(jobs, jobs_limit, |job| {
        let read = fs::read(&job.file).map_err(|e| format!("{}: {e}", job.file.display()))?;
        let (source, encoding) = match String::from_utf8(read) {
            Ok(source) => (source, None),
            Err(error) if ttc::SourceKind::from_tt_path(&job.file).is_some() => {
                return Err(format!(
                    "{}: {}",
                    job.file.display(),
                    std::io::Error::new(std::io::ErrorKind::InvalidData, error.utf8_error())
                ));
            }
            Err(error) => {
                let (source, encoding) = Encoding::decode(error.into_bytes())
                    .map_err(|reason| format!("{}: {reason}", job.file.display()))?;
                (source, Some(encoding))
            }
        };
        let scan = ttc::scan_module_with_kind(
            &source,
            ttc::SourceKind::from_path(&job.file).unwrap_or_default(),
        );
        Ok(Loaded {
            source,
            encoding,
            scan,
        })
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
                std::thread::Builder::new()
                    .stack_size(ttc::stack::COMPILER_STACK_SIZE)
                    .spawn_scoped(scope, move || {
                        let mut done: Vec<(usize, R)> = Vec::new();
                        loop {
                            let i = next.fetch_add(1, Ordering::Relaxed);
                            match items.get(i) {
                                Some(item) => done.push((i, f(item))),
                                None => return done,
                            }
                        }
                    })
                    .unwrap_or_else(|error| {
                        panic!("a compiler worker could not be created: {error}")
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Encoding {
    Utf16Le,
    Utf16Be,
    Bytes,
}

const STRAY_BYTE: u32 = 0xF700;

impl Encoding {
    fn decode(bytes: Vec<u8>) -> Result<(String, Encoding), String> {
        let utf16 = |rest: &[u8], unit: fn([u8; 2]) -> u16, encoding| {
            if !rest.len().is_multiple_of(2) {
                return Err("a UTF-16 file with an odd number of bytes".to_string());
            }
            let units: Vec<u16> = rest
                .chunks(2)
                .map(|pair| unit([pair[0], pair[1]]))
                .collect();
            String::from_utf16(&units)
                .map(|text| (text, encoding))
                .map_err(|_| "a UTF-16 file with an unpaired surrogate".to_string())
        };
        match bytes.as_slice() {
            [0xFF, 0xFE, rest @ ..] => return utf16(rest, u16::from_le_bytes, Encoding::Utf16Le),
            [0xFE, 0xFF, rest @ ..] => return utf16(rest, u16::from_be_bytes, Encoding::Utf16Be),
            _ => {}
        }
        let mut text = String::with_capacity(bytes.len());
        for chunk in bytes.utf8_chunks() {
            if chunk
                .valid()
                .chars()
                .any(|c| (STRAY_BYTE..STRAY_BYTE + 0x100).contains(&u32::from(c)))
            {
                return Err(
                    "a file that is not UTF-8 and also holds U+F700–U+F7FF cannot be passed \
                     through byte for byte — save it as UTF-8"
                        .to_string(),
                );
            }
            text.push_str(chunk.valid());
            for byte in chunk.invalid() {
                text.extend(char::from_u32(STRAY_BYTE + u32::from(*byte)));
            }
        }
        Ok((text, Encoding::Bytes))
    }

    pub(super) fn encode(self, text: &str) -> Vec<u8> {
        match self {
            Encoding::Utf16Le => [0xFF, 0xFE]
                .into_iter()
                .chain(text.encode_utf16().flat_map(u16::to_le_bytes))
                .collect(),
            Encoding::Utf16Be => [0xFE, 0xFF]
                .into_iter()
                .chain(text.encode_utf16().flat_map(u16::to_be_bytes))
                .collect(),
            Encoding::Bytes => {
                let mut bytes = Vec::with_capacity(text.len());
                for c in text.chars() {
                    match u32::from(c).checked_sub(STRAY_BYTE) {
                        Some(byte) if byte < 0x100 => bytes.push(byte as u8),
                        _ => bytes.extend_from_slice(c.encode_utf8(&mut [0; 4]).as_bytes()),
                    }
                }
                bytes
            }
        }
    }
}
