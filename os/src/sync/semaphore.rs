//! Semaphore

use crate::sync::UPSafeCell;
use crate::task::{block_current_and_run_next, current_task, wakeup_task, TaskControlBlock};
use crate::task::{current_process};
use alloc::{collections::VecDeque, sync::Arc};

/// semaphore structure
pub struct Semaphore {
    /// semaphore inner
    pub inner: UPSafeCell<SemaphoreInner>,
    resource_idx: usize,
}

pub struct SemaphoreInner {
    pub count: isize,
    pub wait_queue: VecDeque<Arc<TaskControlBlock>>,
}

impl Semaphore {
    /// Create a new semaphore
    pub fn new(res_count: usize) -> Self {
        trace!("kernel: Semaphore::new");
        let process = current_process();
        let mut inner = process.inner_exclusive_access();
        inner.available.push(res_count);
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
                UPSafeCell::new(SemaphoreInner {
                    count: res_count as isize,
                    wait_queue: VecDeque::new(),
                })
            },
            resource_idx
        }
    }

    /// up operation of semaphore
    pub fn up(&self) {
        trace!("kernel: Semaphore::up");
        let mut inner = self.inner.exclusive_access();
        inner.count += 1;
        let process = current_process();
        let tid = current_task().unwrap().get_tid();
        process.dealloc_values(tid, self.resource_idx);
        
        if inner.count <= 0 {
            if let Some(task) = inner.wait_queue.pop_front() {
                println!("thread {} is waked up", task.get_tid());
                wakeup_task(task);
            }
        }
    }

    /// down operation of semaphore
    pub fn down(&self) -> isize {
        trace!("kernel: Semaphore::down");
        let process = current_process();
        let tid = current_task().unwrap().get_tid();
        process.increase_need(1, tid, self.resource_idx);

        let mut able_to_lock = true;
        if process.deadlock_is_enbale() {
            able_to_lock = process.deadlock_is_safe(self.resource_idx);
        }

        if able_to_lock {
            let mut inner = self.inner.exclusive_access();
            inner.count -= 1;
            if inner.count < 0 {
                println!("thread {} is append to queue.", tid);
                inner.wait_queue.push_back(current_task().unwrap());
                drop(inner);
                block_current_and_run_next();
            } else {
                drop(inner);
                // Alloc the value of deadlock_detect
                process.alloc_values(tid, self.resource_idx);
            }
            0
        } else {
            -0xDEAD
        }
    }
}
