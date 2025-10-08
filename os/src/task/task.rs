//! Types related to task management

use super::TaskContext;

/// The task control block (TCB) of a task.
#[derive(Copy, Clone)]
pub struct TaskControlBlock {
    /// The task status in it's lifecycle
    pub task_status: TaskStatus,
    /// The task context
    pub task_cx: TaskContext,
    /// Using HashMap maybe better. I tried array, but the memory is too much.
    /// sys_write
    pub sys_write_times: usize,
    /// sys_exit
    pub sys_exit_times: usize,
    /// sys_yield
    pub sys_yield_times: usize,
    /// sys_get_time
    pub sys_get_time_times: usize,
    /// sys_trace
    pub sys_trace_times: usize,
}

impl TaskControlBlock {
    /// Initialize a TaskControlBlock
    pub fn new() -> Self {
        TaskControlBlock { 
            task_status: TaskStatus::UnInit, 
            task_cx: TaskContext::zero_init(), 
            sys_write_times: 0, 
            sys_exit_times: 0, 
            sys_yield_times: 0, 
            sys_get_time_times: 0, 
            sys_trace_times: 0, 
        }
    }
}

/// The status of a task
#[derive(Copy, Clone, PartialEq)]
pub enum TaskStatus {
    /// uninitialized
    UnInit,
    /// ready to run
    Ready,
    /// running
    Running,
    /// exited
    Exited,
}
