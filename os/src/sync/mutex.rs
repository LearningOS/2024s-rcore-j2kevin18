//! Mutex (spin-like and blocking(sleep))

use super::UPSafeCell;
use crate::task::TaskControlBlock;
use crate::task::{block_current_and_run_next, suspend_current_and_run_next};
use crate::task::{current_task, wakeup_task};
use alloc::{collections::VecDeque, sync::Arc};

/// Mutex trait
pub trait Mutex: Sync + Send {
    /// Lock the mutex
    fn lock(&self);
    /// Unlock the mutex
    fn unlock(&self);
    /// update the mutex
    fn update(&self);
    ///whether locked or not
    fn is_locked(&self) -> bool;
}

/// Spinlock Mutex struct
pub struct MutexSpin {
    locked: UPSafeCell<bool>,
    id: usize,
}

impl MutexSpin {
    /// Create a new spinlock mutex
    pub fn new(_id: usize) -> Self {
        Self {
            locked: unsafe { UPSafeCell::new(false) },
            id: _id
        }
    }
}

impl Mutex for MutexSpin {
    /// Lock the spinlock mutex
    fn lock(&self) {
        trace!("kernel: MutexSpin::lock");
        loop {
            let mut locked = self.locked.exclusive_access();
            if *locked {
                drop(locked);
                suspend_current_and_run_next();
                continue;
            } else {
                let cur_task =current_task().unwrap();
                let mut current_task_inner = cur_task.inner_exclusive_access();
                current_task_inner.mutex_alloc[self.id]=1;
                current_task_inner.mutex_need[self.id]=0;
                drop(current_task_inner);
                *locked = true;
                return;
            }
        }
    }

    fn unlock(&self) {
        trace!("kernel: MutexSpin::unlock");
        let mut locked = self.locked.exclusive_access();
        let cur_task=current_task().unwrap();
        let mut current_task_inner = cur_task.inner_exclusive_access();
        current_task_inner.mutex_alloc[self.id]=0;
        drop(current_task_inner);
        *locked = false;
    }

    /// cerify the mutex
    fn is_locked(&self) -> bool {
        let locked = self.locked.exclusive_access();
       *locked
    }

    fn update(&self){
        let locked=self.locked.exclusive_access();
        let current_task=current_task().unwrap();
        if *locked
        {
            current_task.inner_exclusive_access().mutex_need[self.id]=1;
        }
        else {
            current_task.inner_exclusive_access().mutex_alloc[self.id]=1;
            current_task.inner_exclusive_access().mutex_need[self.id]=0;
        }
    }
}

/// Blocking Mutex struct
pub struct MutexBlocking {
    inner: UPSafeCell<MutexBlockingInner>,
    id: usize,
}

pub struct MutexBlockingInner {
    locked: bool,
    wait_queue: VecDeque<Arc<TaskControlBlock>>,
}

impl MutexBlocking {
    /// Create a new blocking mutex
    pub fn new(_id: usize) -> Self {
        trace!("kernel: MutexBlocking::new");
        Self {
            inner: unsafe {
                UPSafeCell::new(MutexBlockingInner {
                    locked: false,
                    wait_queue: VecDeque::new(),
                })
            },
            id: _id,
        }
    }
}

impl Mutex for MutexBlocking {
    /// lock the blocking mutex
    fn lock(&self) {
        trace!("kernel: MutexBlocking::lock");
        let mut mutex_inner = self.inner.exclusive_access();
        if mutex_inner.locked {
            mutex_inner.wait_queue.push_back(current_task().unwrap());
            drop(mutex_inner);
            block_current_and_run_next();
        } else {
            mutex_inner.locked = true;
        }
    }

    /// unlock the blocking mutex
    fn unlock(&self) {
        trace!("kernel: MutexBlocking::unlock");
        let mut mutex_inner = self.inner.exclusive_access();
        let current_task=current_task().unwrap();
        assert!(mutex_inner.locked);
        if let Some(waking_task) = mutex_inner.wait_queue.pop_front() {
            waking_task.inner_exclusive_access().mutex_need[self.id] = 0;
            waking_task.inner_exclusive_access().mutex_alloc[self.id] = 1;
            current_task.inner_exclusive_access().mutex_alloc[self.id] = 0;
            wakeup_task(waking_task);
        } else {
            mutex_inner.locked = false;
        }
    }

    /// cerify the blocking mutex
    fn is_locked(&self) -> bool {
        self.inner.exclusive_access().locked
    }

    ///update related matrices
    fn update(&self){
        let inner=self.inner.exclusive_access();
        let current_task=current_task().unwrap();
        if inner.locked
        {
            current_task.inner_exclusive_access().mutex_need[self.id]=1;
        }
        else {
            current_task.inner_exclusive_access().mutex_alloc[self.id]=1;
            current_task.inner_exclusive_access().mutex_need[self.id]=0;
        }
    }
}
