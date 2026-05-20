use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

static UIPI_DEDICATED_CORE: AtomicUsize = AtomicUsize::new(usize::MAX);

pub fn init_uipi_core() -> usize {
    let total_cpus = std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(1);

    let core = std::env::var("UINTR_AFFINITY_CORE")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or_else(|| {
            if total_cpus > 1 {
                total_cpus - 1
            } else {
                0
            }
        });

    let core = core.min(total_cpus - 1);
    UIPI_DEDICATED_CORE.store(core, Ordering::Relaxed);
    tracing::info!(
        "CPU affinity: UIPI dedicated core = {} (total CPUs: {})",
        core, total_cpus
    );
    core
}

pub fn get_uipi_core() -> usize {
    UIPI_DEDICATED_CORE.load(Ordering::Relaxed)
}

pub fn pin_current_thread_to_core(core: usize) {
    let mut cpuset: libc::cpu_set_t = unsafe { std::mem::zeroed() };
    unsafe { libc::CPU_ZERO(&mut cpuset); }
    unsafe { libc::CPU_SET(core, &mut cpuset); }

    let tid = unsafe { libc::gettid() };
    let result = unsafe {
        libc::sched_setaffinity(
            tid,
            std::mem::size_of::<libc::cpu_set_t>(),
            &cpuset,
        )
    };

    if result != 0 {
        tracing::warn!(
            "Failed to pin thread to core {}: {}",
            core,
            std::io::Error::last_os_error()
        );
    } else {
        tracing::info!("Thread pinned to core {}", core);
    }
}

pub fn build_tokio_worker_affinity(uipi_core: usize) -> impl Fn() + Send + Sync + Clone + 'static {
    let total_cpus = std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(1);
    let worker_cores: Vec<usize> = (0..total_cpus).filter(|&c| c != uipi_core).collect();
    let next_core = Arc::new(AtomicUsize::new(0));
    let worker_count = worker_cores.len().max(1);

    tracing::info!(
        "tokio worker affinity: cores {:?} ({} workers, excluding UIPI core {})",
        worker_cores,
        worker_count,
        uipi_core,
    );

    move || {
        let idx = next_core.fetch_add(1, Ordering::Relaxed) % worker_count;
        let core = worker_cores[idx];
        pin_current_thread_to_core(core);
    }
}