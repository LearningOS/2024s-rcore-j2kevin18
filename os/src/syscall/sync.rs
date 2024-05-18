use crate::sync::{Condvar, Mutex, MutexBlocking, MutexSpin, Semaphore};
use crate::task::{block_current_and_run_next, current_process, current_task};
use crate::timer::{add_timer, get_time_ms};
use alloc::sync::Arc;
use alloc::vec::Vec;
/// sleep syscall
pub fn sys_sleep(ms: usize) -> isize {
    trace!(
        "kernel:pid[{}] tid[{}] sys_sleep",
        current_task().unwrap().process.upgrade().unwrap().getpid(),
        current_task()
            .unwrap()
            .inner_exclusive_access()
            .res
            .as_ref()
            .unwrap()
            .tid
    );
    let expire_ms = get_time_ms() + ms;
    let task = current_task().unwrap();
    add_timer(expire_ms, task);
    block_current_and_run_next();
    0
}
/// mutex create syscall
pub fn sys_mutex_create(blocking: bool) -> isize {
    trace!(
        "kernel:pid[{}] tid[{}] sys_mutex_create",
        current_task().unwrap().process.upgrade().unwrap().getpid(),
        current_task()
            .unwrap()
            .inner_exclusive_access()
            .res
            .as_ref()
            .unwrap()
            .tid
    );
    let process = current_process();
    let mut process_inner = process.inner_exclusive_access();
    let final_id = process_inner.mutex_list.len();
    let mutex: Option<Arc<dyn Mutex>> = if !blocking {
        Some(Arc::new(MutexSpin::new(final_id)))
    } else {
        Some(Arc::new(MutexBlocking::new(final_id)))
    };
    if let Some(id) = process_inner
        .mutex_list
        .iter()
        .enumerate()
        .find(|(_, item)| item.is_none())
        .map(|(id, _)| id)
    {
        process_inner.mutex_list[id] = mutex;
        let task_count = process_inner.tasks.len();
        for _i in 0..task_count {
            let task = process_inner.get_task(_i);
            let mut task_inner = task.inner_exclusive_access();
            task_inner.mutex_alloc[id] = 0;
            task_inner.mutex_need[id] = 0;
        }
        id as isize
    } else {
        process_inner.mutex_list.push(mutex);
        let task_count = process_inner.tasks.len();
        for _i in 0..task_count {
            let task = process_inner.get_task(_i);
            let mut task_inner = task.inner_exclusive_access();
            task_inner.mutex_alloc.push(0);
            task_inner.mutex_need.push(0);
        }
        final_id as isize 
    }
}
///mutex deadlock detection
pub fn is_dead_mutex(detect: usize) -> bool {
    if detect == 0 {
        return false;
    }
    if detect != 1 {
        return true;
    } //error! Not supposed to have detect value like this
    let process = current_process();
    let mut inner = process.inner_exclusive_access();
    let task_count=inner.tasks.len();
    let mut work: Vec<usize> = Vec::new();
    let mut finish: Vec<bool> = Vec::new();

    for _i in 0..task_count {
        finish.push(false);
    }

    for i in 0..inner.mutex_list.len(){
        if let Some(mtx) = &mut inner.mutex_list[i]{
            if !mtx.is_locked(){
                work.push(1);
                continue;
            }
        }
        work.push(0);
    }
    
    loop {
        let mut exitable = true;
        let inner_tasks=&mut inner.tasks;
        for i in 0..task_count{
            if finish[i]{
                continue;
            }
            if let Some(task)=&mut inner_tasks[i]{
                let mut f=false;
                let task_inner = task.inner_exclusive_access();
                for j in 0..work.len(){
                    if task_inner.mutex_need[j] > work[j]{
                        f = true;
                        break;
                    }
                }
                if f {
                    continue;
                }
                exitable=false;
                finish[i]=true;

                for j in 0..work.len(){
                    work[j] += task_inner.mutex_alloc[j];
                }

                drop(task_inner);
            }
        }
        if exitable{
            break;
        }
    }
    for i in 0..task_count{
        if !finish[i]{
            return true;
        }
    }
    false
}
/// mutex lock syscall
pub fn sys_mutex_lock(mutex_id: usize) -> isize {
    trace!(
        "kernel:pid[{}] tid[{}] sys_mutex_lock",
        current_task().unwrap().process.upgrade().unwrap().getpid(),
        current_task()
            .unwrap()
            .inner_exclusive_access()
            .res
            .as_ref()
            .unwrap()
            .tid
    );
    let process = current_process();
    let process_inner = process.inner_exclusive_access();
    let mutex = Arc::clone(process_inner.mutex_list[mutex_id].as_ref().unwrap());
    let tem = process_inner.detect;
    drop(process_inner);
    drop(process);
    mutex.update();
    if is_dead_mutex(tem) {
        return -0xDEAD;
    }
    mutex.lock();
    0
}
/// mutex unlock syscall
pub fn sys_mutex_unlock(mutex_id: usize) -> isize {
    trace!(
        "kernel:pid[{}] tid[{}] sys_mutex_unlock",
        current_task().unwrap().process.upgrade().unwrap().getpid(),
        current_task()
            .unwrap()
            .inner_exclusive_access()
            .res
            .as_ref()
            .unwrap()
            .tid
    );
    let process = current_process();
    let process_inner = process.inner_exclusive_access();
    let mutex = Arc::clone(process_inner.mutex_list[mutex_id].as_ref().unwrap());
    drop(process_inner);
    drop(process);
    mutex.unlock();
    0
}
/// semaphore create syscall
pub fn sys_semaphore_create(res_count: usize) -> isize {
    trace!(
        "kernel:pid[{}] tid[{}] sys_semaphore_create",
        current_task().unwrap().process.upgrade().unwrap().getpid(),
        current_task()
            .unwrap()
            .inner_exclusive_access()
            .res
            .as_ref()
            .unwrap()
            .tid
    );
    let process = current_process();
    let mut process_inner = process.inner_exclusive_access();
    let id = if let Some(id) = process_inner
        .semaphore_list
        .iter()
        .enumerate()
        .find(|(_, item)| item.is_none())
        .map(|(id, _)| id)
    {
        process_inner.semaphore_list[id] = Some(Arc::new(Semaphore::new(res_count, id)));
        let task_count = process_inner.tasks.len();
        for _i in 0..task_count {
            let task = process_inner.get_task(_i);
            let mut task_inner = task.inner_exclusive_access();
            task_inner.sem_alloc[id] = 0;
            task_inner.sem_need[id] = 0;
        }
        id
    } else {
        let final_id = process_inner.semaphore_list.len();
        process_inner
            .semaphore_list
            .push(Some(Arc::new(Semaphore::new(res_count, final_id))));
        let task_count = process_inner.tasks.len();
        for _i in 0..task_count {
            let task = process_inner.get_task(_i);
            let mut task_inner = task.inner_exclusive_access();
            task_inner.sem_alloc.push(0);
            task_inner.sem_need.push(0);
        }
        final_id
    };
    id as isize
}
///semaphore deadlock detection
pub fn is_dead_sem(detect: usize) -> bool {
    if detect == 0 {
        return false;
    }
    if detect != 1 {
        return true;
    } //error! Not supposed to have detect value like this
    let process = current_process();
    let mut inner = process.inner_exclusive_access();
    let task_count=inner.tasks.len();
    let mut work: Vec<usize> = Vec::new();
    let mut finish: Vec<bool> = Vec::new();

    for _i in 0..task_count {
        finish.push(false);
    }

    for i in 0..inner.semaphore_list.len(){
        if let Some(sem) = &mut inner.semaphore_list[i]{
            if sem.inner.exclusive_access().count > 0{
                work.push(sem.inner.exclusive_access().count as usize);
                continue;
            }
        }
        work.push(0);
    }
    
    loop {
        let mut exitable = true;
        let inner_tasks=&mut inner.tasks;
        for i in 0..task_count{
            if finish[i]{
                continue;
            }
            if let Some(task)=&mut inner_tasks[i]{
                let mut f=false;
                let task_inner = task.inner_exclusive_access();
                for j in 0..work.len(){
                    if task_inner.sem_need[j] > work[j]{
                        f = true;
                        break;
                    }
                }
                if f {
                    continue;
                }
                exitable=false;
                finish[i]=true;

                for j in 0..work.len(){
                    work[j] += task_inner.sem_alloc[j];
                }

                drop(task_inner);
            }
        }
        if exitable{
            break;
        }
    }
    for i in 0..task_count{
        if !finish[i]{
            return true;
        }
    }
    false
}

/// semaphore up syscall
pub fn sys_semaphore_up(sem_id: usize) -> isize {
    trace!(
        "kernel:pid[{}] tid[{}] sys_semaphore_up",
        current_task().unwrap().process.upgrade().unwrap().getpid(),
        current_task()
            .unwrap()
            .inner_exclusive_access()
            .res
            .as_ref()
            .unwrap()
            .tid
    );
    let process = current_process();
    let process_inner = process.inner_exclusive_access();
    let sem = Arc::clone(process_inner.semaphore_list[sem_id].as_ref().unwrap());
    drop(process_inner);
    sem.up();
    0
}
/// semaphore down syscall
pub fn sys_semaphore_down(sem_id: usize) -> isize {
    trace!(
        "kernel:pid[{}] tid[{}] sys_semaphore_down",
        current_task().unwrap().process.upgrade().unwrap().getpid(),
        current_task()
            .unwrap()
            .inner_exclusive_access()
            .res
            .as_ref()
            .unwrap()
            .tid
    );
    let process = current_process();
    let process_inner = process.inner_exclusive_access();
    let sem = Arc::clone(process_inner.semaphore_list[sem_id].as_ref().unwrap());
    let tem = process_inner.detect;
    drop(process_inner);
    sem.update();
    if is_dead_sem(tem){
       return -0xDEAD;
    }
    sem.down();
    0
}
/// condvar create syscall
pub fn sys_condvar_create() -> isize {
    trace!(
        "kernel:pid[{}] tid[{}] sys_condvar_create",
        current_task().unwrap().process.upgrade().unwrap().getpid(),
        current_task()
            .unwrap()
            .inner_exclusive_access()
            .res
            .as_ref()
            .unwrap()
            .tid
    );
    let process = current_process();
    let mut process_inner = process.inner_exclusive_access();
    let id = if let Some(id) = process_inner
        .condvar_list
        .iter()
        .enumerate()
        .find(|(_, item)| item.is_none())
        .map(|(id, _)| id)
    {
        process_inner.condvar_list[id] = Some(Arc::new(Condvar::new()));
        id
    } else {
        process_inner
            .condvar_list
            .push(Some(Arc::new(Condvar::new())));
        process_inner.condvar_list.len() - 1
    };
    id as isize
}
/// condvar signal syscall
pub fn sys_condvar_signal(condvar_id: usize) -> isize {
    trace!(
        "kernel:pid[{}] tid[{}] sys_condvar_signal",
        current_task().unwrap().process.upgrade().unwrap().getpid(),
        current_task()
            .unwrap()
            .inner_exclusive_access()
            .res
            .as_ref()
            .unwrap()
            .tid
    );
    let process = current_process();
    let process_inner = process.inner_exclusive_access();
    let condvar = Arc::clone(process_inner.condvar_list[condvar_id].as_ref().unwrap());
    drop(process_inner);
    condvar.signal();
    0
}
/// condvar wait syscall
pub fn sys_condvar_wait(condvar_id: usize, mutex_id: usize) -> isize {
    trace!(
        "kernel:pid[{}] tid[{}] sys_condvar_wait",
        current_task().unwrap().process.upgrade().unwrap().getpid(),
        current_task()
            .unwrap()
            .inner_exclusive_access()
            .res
            .as_ref()
            .unwrap()
            .tid
    );
    let process = current_process();
    let process_inner = process.inner_exclusive_access();
    let condvar = Arc::clone(process_inner.condvar_list[condvar_id].as_ref().unwrap());
    let mutex = Arc::clone(process_inner.mutex_list[mutex_id].as_ref().unwrap());
    drop(process_inner);
    condvar.wait(mutex);
    0
}
/// enable deadlock detection syscall
///
/// YOUR JOB: Implement deadlock detection, but might not all in this syscall
pub fn sys_enable_deadlock_detect(_enabled: usize) -> isize {
    trace!("kernel: sys_enable_deadlock_detect NOT IMPLEMENTED");
    if _enabled==0 ||_enabled==1{
        let process=current_process();
        let mut _inner=process.inner_exclusive_access();
        _inner.detect=_enabled;
        return 1;
    }
    -1
}
