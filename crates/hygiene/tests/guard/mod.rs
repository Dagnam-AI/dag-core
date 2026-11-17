//! A memory cap for the test binary: the process aborts when the live heap passes `CAP`.
//!
//! `cargo mutants` turns a loop's step into a constant and the loop then pushes forever; without a
//! cap one such mutant reached tens of gigabytes in seconds and took the machine down. Aborting
//! makes the mutant fail its test, which is what "caught" means. No test comes near the cap.
use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;
use std::io::Write;
use std::sync::atomic::{AtomicUsize, Ordering};

/// The most live heap the test binary may hold, in bytes.
const CAP: usize = 1 << 30;

/// Heap bytes live now.
static LIVE: AtomicUsize = AtomicUsize::new(0);

thread_local! {
    /// Heap bytes this thread has allocated net of what it has freed since `reset_high_water`
    /// (negative when it frees what was live before), and the most that figure has been.
    static MINE: Cell<(isize, isize)> = const { Cell::new((0, 0)) };
}

/// Starts a measurement on this thread. Read it with `high_water`. The mark is the thread's own,
/// so another test running on another thread of the same process (`cargo test` runs tests as
/// threads of one) does not count; a measured call that spawns threads would not count their heap.
pub(crate) fn reset_high_water() {
    MINE.set((0, 0));
}

/// The most heap bytes this thread has had live at once beyond what was live at `reset_high_water`.
pub(crate) fn high_water() -> usize {
    usize::try_from(MINE.get().1).unwrap_or(0)
}

/// Records `delta` bytes (negative: freed) on this thread's measurement.
fn track(delta: isize) {
    let (live, peak) = MINE.get();
    let live = live + delta;
    MINE.set((live, peak.max(live)));
}

/// The system allocator with a cap.
pub(crate) struct Capped;

/// Stops the process. Reporting may allocate (the first use of stderr names the thread), so the
/// total is cleared first: left over the cap, that allocation would trip again and recurse until
/// the stack overflowed, hiding the report.
fn trip() -> ! {
    LIVE.store(0, Ordering::Relaxed);
    let _ = std::io::stderr().write_all(b"\ntest binary passed its 1 GiB heap cap: aborting\n");
    std::process::abort()
}

/// A size as a signed number (no allocation is larger than `isize::MAX`).
fn signed(bytes: usize) -> isize {
    isize::try_from(bytes).unwrap_or(isize::MAX)
}

/// Adds `bytes` to the live total and stops the process past the cap.
fn grow(bytes: usize) {
    let live = LIVE.fetch_add(bytes, Ordering::Relaxed) + bytes;
    track(signed(bytes));
    if live > CAP {
        trip();
    }
}

// SAFETY: every call is forwarded to `System` with the caller's own arguments; the counter only
// observes sizes.
unsafe impl GlobalAlloc for Capped {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        grow(layout.size());
        unsafe { System.alloc(layout) }
    }

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        grow(layout.size());
        unsafe { System.alloc_zeroed(layout) }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        LIVE.fetch_sub(layout.size(), Ordering::Relaxed);
        track(-signed(layout.size()));
        unsafe { System.dealloc(ptr, layout) }
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        if new_size >= layout.size() {
            grow(new_size - layout.size());
        } else {
            LIVE.fetch_sub(layout.size() - new_size, Ordering::Relaxed);
            track(-signed(layout.size() - new_size));
        }
        unsafe { System.realloc(ptr, layout, new_size) }
    }
}
