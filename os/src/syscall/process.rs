//! Process management syscalls
use alloc::sync::Arc;

use crate::{
    loader::get_app_data_by_name,
    mm::{translated_refmut, translated_str, translated_byte_buffer},
    task::{
        add_task, current_task, current_user_token, exit_current_and_run_next,
        suspend_current_and_run_next,
    },
    timer::{get_time_ms, get_time_us},
};
use crate::config::PAGE_SIZE;
use crate::mm::MapPermission;

#[repr(C)]
#[derive(Debug)]
pub struct TimeVal {
    pub sec: usize,
    pub usec: usize,
}

/// task exits and submit an exit code
pub fn sys_exit(exit_code: i32) -> ! {
    trace!("kernel:pid[{}] sys_exit", current_task().unwrap().pid.0);
    exit_current_and_run_next(exit_code);
    panic!("Unreachable in sys_exit!");
}

/// current task gives up resources for other tasks
pub fn sys_yield() -> isize {
    trace!("kernel:pid[{}] sys_yield", current_task().unwrap().pid.0);
    suspend_current_and_run_next();
    0
}

pub fn sys_getpid() -> isize {
    trace!("kernel: sys_getpid pid:{}", current_task().unwrap().pid.0);
    current_task().unwrap().pid.0 as isize
}

pub fn sys_fork() -> isize {
    trace!("kernel:pid[{}] sys_fork", current_task().unwrap().pid.0);
    let current_task = current_task().unwrap();
    let new_task = current_task.fork();
    let new_pid = new_task.pid.0;
    // modify trap context of new_task, because it returns immediately after switching
    let trap_cx = new_task.inner_exclusive_access().get_trap_cx();
    // we do not have to move to next instruction since we have done it before
    // for child process, fork returns 0
    trap_cx.x[10] = 0;
    // add new task to scheduler
    add_task(new_task);
    new_pid as isize
}

pub fn sys_exec(path: *const u8) -> isize {
    trace!("kernel:pid[{}] sys_exec", current_task().unwrap().pid.0);
    let token = current_user_token();
    let path = translated_str(token, path);
    if let Some(data) = get_app_data_by_name(path.as_str()) {
        let task = current_task().unwrap();
        task.exec(data);
        0
    } else {
        -1
    }
}

/// If there is not a child process whose pid is same as given, return -1.
/// Else if there is a child process but it is still running, return -2.
pub fn sys_waitpid(pid: isize, exit_code_ptr: *mut i32) -> isize {
    trace!("kernel::pid[{}] sys_waitpid [{}]", current_task().unwrap().pid.0, pid);
    let task = current_task().unwrap();
    // find a child process

    // ---- access current PCB exclusively
    let mut inner = task.inner_exclusive_access();
    if !inner
        .children
        .iter()
        .any(|p| pid == -1 || pid as usize == p.getpid())
    {
        return -1;
        // ---- release current PCB
    }
    let pair = inner.children.iter().enumerate().find(|(_, p)| {
        // ++++ temporarily access child PCB exclusively
        p.inner_exclusive_access().is_zombie() && (pid == -1 || pid as usize == p.getpid())
        // ++++ release child PCB
    });
    if let Some((idx, _)) = pair {
        let child = inner.children.remove(idx);
        // confirm that child will be deallocated after being removed from children list
        assert_eq!(Arc::strong_count(&child), 1);
        let found_pid = child.getpid();
        // ++++ temporarily access child PCB exclusively
        let exit_code = child.inner_exclusive_access().exit_code;
        // ++++ release child PCB
        *translated_refmut(inner.memory_set.token(), exit_code_ptr) = exit_code;
        found_pid as isize
    } else {
        -2
    }
    // ---- release current PCB automatically
}

/// YOUR JOB: get time with second and microsecond
/// HINT: You might reimplement it with virtual memory management.
/// HINT: What if [`TimeVal`] is splitted by two pages ?
pub fn sys_get_time(ts: *mut TimeVal, _tz: usize) -> isize {
    trace!(
        "kernel:pid[{}] sys_get_time NOT IMPLEMENTED",
        current_task().unwrap().pid.0
    );
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

/// YOUR JOB: Implement mmap.
pub fn sys_mmap(start: usize, len: usize, prot: usize) -> isize {
    trace!("kernel:pid[{}] sys_mmap", current_task().unwrap().pid.0);
    if start % PAGE_SIZE != 0 {
        return -1;
    }
    // prot must contain at least one of R/W/X and must not contain unsupported bits
    // Accept only bits 0..2 (R/W/X). Any other bits (e.g., 8) are invalid.
    if (prot & 0x7) == 0 || (prot & !0x7) != 0 {
        return -1;
    }
    let page_count = (len + PAGE_SIZE - 1) / PAGE_SIZE;
    let end = start + page_count * PAGE_SIZE;

    // build MapPermission from prot bits (R=bit0, W=bit1, X=bit2)
    let mut perm = MapPermission::empty() | MapPermission::U;
    if prot & (1 << 0) != 0 { perm |= MapPermission::R; }
    if prot & (1 << 1) != 0 { perm |= MapPermission::W; }
    if prot & (1 << 2) != 0 { perm |= MapPermission::X; }

    let task = current_task().unwrap();
    let mut inner = task.inner_exclusive_access();
    inner.memory_set.insert_framed_area_checked(start.into(), end.into(), perm)
}

/// YOUR JOB: Implement munmap.
pub fn sys_munmap(start: usize, len: usize) -> isize {
    trace!("kernel:pid[{}] sys_munmap", current_task().unwrap().pid.0);
    if start % PAGE_SIZE != 0 {
        return -1;
    }
    let page_count = (len + PAGE_SIZE - 1) / PAGE_SIZE;
    let end = start + page_count * PAGE_SIZE;
    let task = current_task().unwrap();
    let mut inner = task.inner_exclusive_access();
    inner.memory_set.remove_area_with_start_end(start.into(), end.into())
}

/// change data segment size
pub fn sys_sbrk(size: i32) -> isize {
    trace!("kernel:pid[{}] sys_sbrk", current_task().unwrap().pid.0);
    if let Some(old_brk) = current_task().unwrap().change_program_brk(size) {
        old_brk as isize
    } else {
        -1
    }
}

/// YOUR JOB: Implement spawn.
/// HINT: fork + exec =/= spawn
pub fn sys_spawn(path: *const u8) -> isize {
    trace!("kernel:pid[{}] sys_spawn", current_task().unwrap().pid.0);
    let current_task = current_task().unwrap();
    let token = current_user_token();
    let path = translated_str(token, path);

    if let Some(data) = get_app_data_by_name(path.as_str()) {
        let new_task = current_task.spawn(data);
        add_task(new_task);
        new_task.pid.0 as isize
    } else {
        -1
    }
}

// YOUR JOB: Set task priority.
pub fn sys_set_priority(prio: isize) -> isize {
    trace!(
        "kernel:pid[{}] sys_set_priority",
        current_task().unwrap().pid.0
    );
    if prio >= 2 {
        let current_task = current_task().unwrap();
        current_task.inner_exclusive_access().prio = prio;
        prio
    } else {
        -1
    }
}
