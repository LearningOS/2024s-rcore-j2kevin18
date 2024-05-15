//! Process management syscalls
use crate::{
    config::MAX_SYSCALL_NUM,
    task::{
        change_program_brk, exit_current_and_run_next, suspend_current_and_run_next, TaskStatus,
        map, unmap, current_user_token, get_syscall_times, get_first_time,
    },
    mm::{get_easy_ptr_from_token, VirtAddr, MapPermission},
    timer::get_time_us,
};

#[repr(C)]
#[derive(Debug)]
pub struct TimeVal {
    pub sec: usize,
    pub usec: usize,
}

/// Task information
#[allow(dead_code)]
pub struct TaskInfo {
    /// Task status in it's life cycle
    status: TaskStatus,
    /// The numbers of syscall called by task
    syscall_times: [u32; MAX_SYSCALL_NUM],
    /// Total running time of task
    time: usize,
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
pub fn sys_get_time(_ts: *mut TimeVal, _tz: usize) -> isize {
    trace!("kernel: sys_get_time");
    let _us = get_time_us();
    let token = current_user_token();
    let ts: &mut TimeVal = get_easy_ptr_from_token(token, _ts as *const u8);
    *ts = TimeVal {
        sec: _us / 1_000_000,
        usec: _us%1_000_000,
    };
    0
}

/// YOUR JOB: Finish sys_task_info to pass testcases
/// HINT: You might reimplement it with virtual memory management.
/// HINT: What if [`TaskInfo`] is splitted by two pages ?
pub fn sys_task_info(_ti: *mut TaskInfo) -> isize {
    trace!("kernel: sys_task_info NOT IMPLEMENTED YET!");
    let _us = get_time_us();
    let token = current_user_token();
    let ti: &mut TaskInfo = get_easy_ptr_from_token(token, _ti as *const u8);
    *ti = TaskInfo {
       status: TaskStatus::Running,
       syscall_times: get_syscall_times(),
       time: (_us - get_first_time()) / 1000
    };
    0
}

// YOUR JOB: Implement mmap.
pub fn sys_mmap(_start: usize, _len: usize, _port: usize) -> isize {
    trace!("kernel: sys_mmap NOT IMPLEMENTED YET!");
    let start_addr = VirtAddr::from(_start);
    let start_offset : usize = start_addr.page_offset();
    if start_offset>0 || _port & !0x7 != 0 || _port & 0x7 == 0 {
        return -1;
    }

    let end_addr:VirtAddr = VirtAddr::from(_start + _len);
    let end_page_num = end_addr.ceil();
    let mut permission=MapPermission::U;
    if _port&0x1!=0 {
        permission |= MapPermission::R;
    }
    if _port&0x2!=0 {
        permission |= MapPermission::W;
    }
    if _port&0x4!=0 {
        permission |= MapPermission::X;
    }
    if !map(start_addr.floor(), end_page_num, permission){
        return -1;
    }
    0
}

// YOUR JOB: Implement munmap.
pub fn sys_munmap(_start: usize, _len: usize) -> isize {
    trace!("kernel: sys_munmap NOT IMPLEMENTED YET!");
    let start_addr=VirtAddr::from(_start);
    let start_offset : usize = start_addr.page_offset();
    if start_offset>0 {
        return -1;
    }
    let end_addr=VirtAddr::from(_start+_len);
    let end_page_num=end_addr.ceil();
    if !unmap(start_addr.floor(),end_page_num){
        return -1;
    }
    0
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
