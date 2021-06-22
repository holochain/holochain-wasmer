#[global_allocator]
static GLOBAL: AllocationTracker = AllocationTracker::new();

use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicIsize, Ordering};

pub mod prelude {
    pub use super::ScopeTracker;
}

pub struct AllocationTracker {
    mem: AtomicIsize,
}

impl AllocationTracker {
    pub const fn new() -> Self {
        AllocationTracker {
            mem: AtomicIsize::new(0),
        }
    }

    fn current_mem(&self) -> isize {
        self.mem.load(Ordering::SeqCst)
    }
}

unsafe impl GlobalAlloc for AllocationTracker {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        self.mem.fetch_add(layout.size() as isize, Ordering::SeqCst);
        System.alloc(layout)
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        self.mem.fetch_sub(layout.size() as isize, Ordering::SeqCst);
        System.dealloc(ptr, layout)
    }
}

pub struct ScopeTracker<'a> {
    at_start: isize,
    name: &'a str,
    file: &'static str,
    line: u32,
    pub quiet: bool,
}

impl<'a> ScopeTracker<'a> {
    pub fn new(name: &'a str, file: &'static str, line: u32, quiet: bool) -> Self {
        Self {
            at_start: GLOBAL.current_mem(),
            name,
            file,
            line,
            quiet,
        }
    }

    pub fn leaked(&self) -> isize {
        let old = self.at_start;
        let new = GLOBAL.current_mem();

        new - old
    }
}

impl Drop for ScopeTracker<'_> {
    fn drop(&mut self) {
        let old = self.at_start;
        let new = GLOBAL.current_mem();
        if !self.quiet {
            if old != new {
                if self.name == "" {
                    eprintln!(
                        "{}:{}: {} bytes escaped scope",
                        self.file,
                        self.line,
                        self.leaked()
                    );
                } else {
                    eprintln!(
                        "{}:{} '{}': {} bytes escaped scope",
                        self.file,
                        self.line,
                        self.name,
                        self.leaked()
                    );
                }
            }
        }
    }
}

#[macro_export]
macro_rules! mem_guard {
    () => {
        mem_guard!("", true)
    };

    ($e:expr) => {
        mem_guard!($e, true)
    };

    ($e:expr, $quiet:expr) => {
        ScopeTracker::new($e, file!(), line!(), $quiet)
    };
}
