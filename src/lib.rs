use futures::executor::block_on;
use futures_util::{SinkExt, StreamExt};
use gio_sys::{g_task_new, g_task_return_boolean, GAsyncReadyCallback, GAsyncResult, GTask};
use glib_sys::{gpointer, GError};
use gobject_sys::{g_object_unref, GObject};
use std::{
    sync::{Arc, Mutex},
    thread,
};

pub struct State {
    sender: Arc<Mutex<futures::channel::mpsc::Sender<SomeMsg>>>,
    workers: std::thread::JoinHandle<()>,
}

impl State {
    pub fn new(
        sender: futures::channel::mpsc::Sender<SomeMsg>,
        workers: std::thread::JoinHandle<()>,
    ) -> Self {
        State {
            sender: Arc::new(Mutex::new(sender)),
            workers,
        }
    }

    pub fn send(&self, msg: SomeMsg) {
        block_on((self.sender.lock().unwrap()).send(msg)).unwrap();
    }

    pub fn join_workers(self) {
        eprintln!(
            "{:?} waiting for Tokio thread to complete",
            std::thread::current()
        );
        self.workers.join().unwrap();
    }
}

#[derive(Debug)]
pub enum SomeMsg {
    Task(Arc<Mutex<usize>>),
    Terminate,
}

#[no_mangle]
pub fn rs_do_something_finish(
    _src: *mut GObject,
    result: *mut GAsyncResult,
    _error: &mut GError,
) -> usize {
    eprintln!(
        "{:?} !rs_do_something_finish() called with result = {:?}",
        std::thread::current(),
        result,
    );

    1
}

async fn rs_worker(mut receiver: futures::channel::mpsc::Receiver<SomeMsg>) {
    loop {
        eprintln!("{:?} thread waiting...", std::thread::current());
        while let Some(msg) = receiver.next().await {
            eprintln!("{:?} msg: {:?}", std::thread::current(), msg);
            match msg {
                SomeMsg::Terminate => return,
                SomeMsg::Task(task) => {
                    // Perform every task in an available worker thread rather than this one
                    tokio::spawn(async move {
                        let task = *task.lock().unwrap() as *mut GTask;
                        eprintln!("{:?} handling task {:?}", std::thread::current(), task);
                        unsafe {
                            g_task_return_boolean(task, 1);
                            g_object_unref(task as *mut gobject_sys::GObject);
                        };
                    });
                }
            }
        }
    }
}

#[no_mangle]
fn rs_init() -> *mut State {
    eprintln!("{:?} enter rs_init()", std::thread::current());

    let (sender, receiver) = futures::channel::mpsc::channel::<SomeMsg>(10240);

    // Spawn a thread to run a Tokio event loop
    let workers = thread::spawn(move || {
        use tokio::runtime::Builder;

        let mut rt = Builder::new()
            .threaded_scheduler()
            .core_threads(3)
            .build()
            .unwrap();
        rt.block_on(rs_worker(receiver));
    });

    let state = Box::new(State::new(sender, workers));

    eprintln!("{:?} exit rs_init()", std::thread::current());

    Box::into_raw(state)
}

#[no_mangle]
pub fn rs_do_something_async(
    state: *mut State,
    callback: GAsyncReadyCallback,
    user_data: gpointer,
) {
    let task: Arc<Mutex<usize>>;
    unsafe {
        let raw_task = g_task_new(
            std::ptr::null_mut::<gobject_sys::GObject>(),
            std::ptr::null_mut::<gio_sys::GCancellable>(),
            callback,
            user_data,
        );

        gobject_sys::g_object_weak_ref(
            raw_task as *mut gobject_sys::GObject,
            Some(rs_task_disposed),
            std::ptr::null_mut::<std::ffi::c_void>(),
        );
        task = Arc::new(Mutex::new(raw_task as usize));
    }

    let new_task = task.clone();

    eprintln!("{:?} Sending task down", std::thread::current());
    unsafe { state.as_ref().unwrap().send(SomeMsg::Task(new_task)) };
    eprintln!("{:?} Sent task down", std::thread::current());
}

#[no_mangle]
pub unsafe fn rs_done(state: *mut State) {
    let state = Box::from_raw(state);
    state.send(SomeMsg::Terminate);
    state.join_workers();
}

#[no_mangle]
unsafe extern "C" fn rs_task_disposed(_user_data: gpointer, defunct_task: *mut GObject) {
    eprintln!(
        "{:?} Task {} disposed",
        std::thread::current(),
        defunct_task as usize
    );
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
