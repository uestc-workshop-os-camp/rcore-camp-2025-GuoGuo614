//! Mutex (spin-like and blocking(sleep))

use super::UPSafeCell;
use crate::task::TaskControlBlock;
use crate::task::{block_current_and_run_next, suspend_current_and_run_next};
use crate::task::{current_task, wakeup_task};
use crate::task::{current_process};
use alloc::{collections::VecDeque, sync::Arc};

/// Mutex trait
pub trait Mutex: Sync + Send {
    /// Lock the mutex
    fn lock(&self) -> isize;
    /// Unlock the mutex
    fn unlock(&self);
}

/// Spinlock Mutex struct
pub struct MutexSpin {
    locked: UPSafeCell<bool>,
    resource_idx: usize,
}

impl MutexSpin {
    /// Create a new spinlock mutex
    pub fn new() -> Self {
        let process = current_process();
        let mut inner = process.inner_exclusive_access();
        inner.available.push(1);
        let resource_idx = inner.available.len() - 1;
        for alloc in inner.allocation.iter_mut() {
            alloc.1.push(0);
        }
        for need in inner.need.iter_mut() {
            need.1.push(0);
        }
        drop(inner);

        Self {
            locked: unsafe { UPSafeCell::new(false) },
            resource_idx
        }
    }
}

impl Mutex for MutexSpin {
    /// Lock the spinlock mutex
    fn lock(&self) -> isize {
        trace!("kernel: MutexSpin::lock");
        loop {
            let process = current_process();
            let tid = current_task().unwrap().get_tid();
            process.increase_need(1, tid, self.resource_idx);

            let mut able_to_lock = true;
            if process.deadlock_is_enbale() {
                able_to_lock = process.deadlock_is_safe(self.resource_idx);
            }

            if able_to_lock {
                let mut locked = self.locked.exclusive_access();
                if *locked {
                    drop(locked);
                    suspend_current_and_run_next();
                    continue;
                } else {
                    // Alloc the value of deadlock_detect
                    process.alloc_values(tid, self.resource_idx);
                   *locked = true;
                    return 0;
                }
            } else {
                return -0xDEAD;
            }
        }
    }

    fn unlock(&self) {
        trace!("kernel: MutexSpin::unlock");
        let process = current_process();
        let tid = current_task().unwrap().get_tid();
        process.dealloc_values(tid, self.resource_idx);

        let mut locked = self.locked.exclusive_access();
        *locked = false;
    }
}

/// Blocking Mutex struct
pub struct MutexBlocking {
    inner: UPSafeCell<MutexBlockingInner>,
    resource_idx: usize,
}

pub struct MutexBlockingInner {
    locked: bool,
    wait_queue: VecDeque<Arc<TaskControlBlock>>,
}

impl MutexBlocking {
    /// Create a new blocking mutex
    pub fn new() -> Self {
        trace!("kernel: MutexBlocking::new");
        let process = current_process();
        let mut inner = process.inner_exclusive_access();
        inner.available.push(1);
        let resource_idx = inner.available.len() - 1;
        for alloc in inner.allocation.iter_mut() {
            alloc.1.push(0);
        }
        for need in inner.need.iter_mut() {
            need.1.push(0);
        }
        drop(inner);
        
        Self {
            inner: unsafe {
                UPSafeCell::new(MutexBlockingInner {
                    locked: false,
                    wait_queue: VecDeque::new(),
                })
            },
            resource_idx
        }
    }
}

impl Mutex for MutexBlocking {
    /// lock the blocking mutex
    fn lock(&self) -> isize {
        trace!("kernel: MutexBlocking::lock");
        let process = current_process();
        let tid = current_task().unwrap().get_tid();
        process.increase_need(1, tid, self.resource_idx);

        let mut able_to_lock = true;
        if process.deadlock_is_enbale() {
            able_to_lock = process.deadlock_is_safe(self.resource_idx);
        }

        if able_to_lock {
            let mut mutex_inner = self.inner.exclusive_access();
            if mutex_inner.locked {
                mutex_inner.wait_queue.push_back(current_task().unwrap());
                drop(mutex_inner);
                block_current_and_run_next();
            } else {
                // Alloc the value of deadlock_detect
                process.alloc_values(tid, self.resource_idx);
                mutex_inner.locked = true;
            }
            0
        } else {
            -0xDEAD
        }
    }

    /// unlock the blocking mutex
    fn unlock(&self) {
        trace!("kernel: MutexBlocking::unlock");
        let process = current_process();
        let tid = current_task().unwrap().get_tid();
        process.dealloc_values(tid, self.resource_idx);
        
        let mut mutex_inner = self.inner.exclusive_access();
        assert!(mutex_inner.locked);
        if let Some(waking_task) = mutex_inner.wait_queue.pop_front() {
            wakeup_task(waking_task);
        } else {
            mutex_inner.locked = false;
        }
    }
}
