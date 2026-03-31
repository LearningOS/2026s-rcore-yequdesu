//! Types related to task management

use super::TaskContext;

use crate::config::MAX_SYSCALL_NUM;

/// The task control block (TCB) of a task.
#[derive(Copy, Clone)]
pub struct TaskControlBlock {
    /// The task status in it's lifecycle
    pub task_status: TaskStatus,
    /// The task context
    pub task_cx: TaskContext,
    /// The times of syscalls
    pub syscall_cnt: [usize; MAX_SYSCALL_NUM],
}

impl TaskControlBlock {
    /// increment syscall count by 1
    pub fn inc_syscall(&mut self, id: usize) {
        if id < MAX_SYSCALL_NUM {
            self.syscall_cnt[id] += 1;
        }
    }
    /// get the cnt of syscalls
    pub fn get_syscall_cnt(&self, id: usize) -> usize {
        if id < MAX_SYSCALL_NUM {
            self.syscall_cnt[id]
        } else {
            0
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
