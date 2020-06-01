use futures::executor::block_on;
use futures_util::{SinkExt, StreamExt};
use gio_sys::{
    g_cancellable_new, g_task_new, g_task_return_boolean, GAsyncReadyCallback, GAsyncResult, GTask,
};
use glib_sys::{gpointer, GError};
use gobject_sys::{g_object_new, g_object_unref, GObject, G_TYPE_OBJECT};
use std::{
    sync::{Arc, Mutex},
    thread,
};

static mut SENDER: Option<Mutex<futures::channel::mpsc::Sender<SomeMsg>>> = None;

#[derive(Debug)]
enum SomeMsg {
    Task(Arc<Mutex<usize>>),
    Terminate,
}

#[no_mangle]
pub fn rs_do_something_finish(
    _src: GObject,
    result: *mut gio_sys::GAsyncResult,
    _error: &mut GError,
) -> usize {
    eprintln!(
        "{:?} !rs_do_something_finish() called with result = {:?}",
        std::thread::current(),
        result,
    );
    // TODO: cast GAsyncResult back to the task and unref it?

    1
}

async fn rs_worker(mut receiver: futures::channel::mpsc::Receiver<SomeMsg>) {
    loop {
        eprintln!("{:?} thread waiting...", std::thread::current());
        while let Some(msg) = receiver.next().await {
            eprintln!("{:?} msg: {:?}", std::thread::current(), msg);
            match msg {
                SomeMsg::Terminate => break,
                SomeMsg::Task(task) => {
                    // Perform every task in an available worker thread rather than this one
                    tokio::spawn(async move {
                        let task = *task.lock().unwrap() as *mut GTask;
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
fn rs_init() {
    eprintln!("{:?} enter rs_init()", std::thread::current());

    let (_sender, receiver) = futures::channel::mpsc::channel::<SomeMsg>(10240);

    unsafe { SENDER = Some(Mutex::new(_sender)) };

    // Spawn a thread to run a Tokio event loop
    thread::spawn(move || {
        use tokio::runtime::Builder;

        let mut rt = Builder::new()
            .threaded_scheduler()
            .core_threads(3)
            .build()
            .unwrap();
        rt.block_on(rs_worker(receiver));
    });

    eprintln!("{:?} exit rs_init()", std::thread::current());
}

fn get_sender() -> futures::channel::mpsc::Sender<SomeMsg> {
    unsafe {
        match &SENDER {
            Some(sender) => {
                let guard = sender.lock().unwrap();
                guard.clone()
            }
            None => panic!("not initialized"),
        }
    }
}

#[no_mangle]
pub fn rs_do_something_async(callback: GAsyncReadyCallback, user_data: gpointer) {
    let task: Arc<Mutex<usize>>;
    unsafe {
        let raw_task = g_task_new(
            0 as *mut gobject_sys::GObject,
            0 as *mut gio_sys::GCancellable,
            callback,
            user_data,
        );

        gobject_sys::g_object_weak_ref(
            raw_task as *mut gobject_sys::GObject,
            Some(rs_task_disposed),
            0 as *mut std::ffi::c_void,
        );
        task = Arc::new(Mutex::new(raw_task as usize));
    }

    let new_task = task.clone();

    let mut sender = get_sender();
    eprintln!("{:?} Sending task down", std::thread::current());
    block_on(sender.send(SomeMsg::Task(new_task))).unwrap();
    eprintln!("{:?} Sent task down", std::thread::current());
}

#[no_mangle]
pub fn rs_done() {
    let mut sender = get_sender();
    block_on(sender.send(SomeMsg::Terminate)).unwrap();
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
