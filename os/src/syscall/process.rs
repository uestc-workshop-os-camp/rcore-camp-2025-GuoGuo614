//! Process management syscalls
use crate::task::{
    change_program_brk, 
    exit_current_and_run_next, 
    suspend_current_and_run_next,
    trace_syscall,
    current_user_token,
    mmap,
    munmap
};

use crate::mm::{translated_byte_buffer, translated_const_ptr, translated_mut_ptr};
use crate::timer::{get_time_ms, get_time_us};

#[repr(C)]
#[derive(Debug)]
pub struct TimeVal {
    pub sec: usize,
    pub usec: usize,
}

/// task exits and submit an exit code
pub fn sys_exit(_exit_code: i32) -> ! {
    trace!("kernel: sys_exit");
    exit_current_and_run_next();
    panic!("Unreachable in sys_exit!");
}

/// current task gives up resources for other tasks
pub fn sys_yield() -> isize {
    trace!("kernel: sys_yield");
    suspend_current_and_run_next();
    0
}

/// YOUR JOB: get time with second and microsecond
/// HINT: You might reimplement it with virtual memory management.
/// HINT: What if [`TimeVal`] is splitted by two pages ?
pub fn sys_get_time(ts: *mut TimeVal, _tz: usize) -> isize {
    trace!("kernel: sys_get_time");
    let token = current_user_token();
    let len = core::mem::size_of::<TimeVal>();
    let buf = translated_byte_buffer(token, ts as *mut u8, len);
    let time = TimeVal {
        sec: get_time_ms() / 1000,
        usec: get_time_us() % 1_000_000
    };
    let bytes = unsafe {
        core::slice::from_raw_parts(&time as *const TimeVal as *const u8, len)
    };
    let mut offset = 0;
    for seg in buf {
        let len = seg.len().min(bytes.len() - offset);
        seg[..len].copy_from_slice(&bytes[offset..offset + len]);
        offset += len;
    }
    0
}

/// Finish sys_trace to pass testcases
/// HINT: You might reimplement it with virtual memory management.
pub fn sys_trace(trace_request: usize, id: usize, data: usize) -> isize {
    trace!("kernel: sys_trace");
    let token = current_user_token();
    match trace_request {
        0 => {
            let ptr = translated_const_ptr(token, id as *const u8);
            match ptr {
                None => -1_isize,
                Some(ptr) => unsafe {
                    *ptr as isize
                }
            }
        }
        1 => {
            let ptr = translated_mut_ptr(token, id as *mut u8);
            match ptr {
                None => -1_isize,
                Some(ptr) => unsafe {
                    *ptr = data as u8;
                    0
                }
            }
        }
        2 => { trace_syscall(id) }
        _ => { -1 }
    }
}

// YOUR JOB: Implement mmap.
pub fn sys_mmap(start: usize, len: usize, prot: usize) -> isize {
    trace!("kernel: sys_mmap");
    if prot & !0x7 != 0 || prot & 0x7 == 0 {
        return -1;
    }
    mmap(start, len, prot)
}

// YOUR JOB: Implement munmap.
pub fn sys_munmap(start: usize, len: usize) -> isize {
    trace!("kernel: sys_munmap NOT IMPLEMENTED YET!");
    munmap(start, len)
}
/// change data segment size
pub fn sys_sbrk(size: i32) -> isize {
    trace!("kernel: sys_sbrk");
    if let Some(old_brk) = change_program_brk(size) {
        old_brk as isize
    } else {
        -1
    }
}
