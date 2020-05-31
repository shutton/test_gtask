use futures::executor::block_on;
use futures_util::{SinkExt, StreamExt};
use gio_sys::{
    g_cancellable_new, g_task_new, g_task_return_boolean, GAsyncReadyCallback, GAsyncResult, GTask,
};
use glib_sys::{gpointer, GError};
use gobject_sys::{g_object_new, GObject, G_TYPE_OBJECT};
use std::{
    sync::{Arc, Mutex},
    thread,
};

static mut SENDER: Option<futures::channel::mpsc::Sender<SomeMsg>> = None;

#[derive(Debug)]
enum SomeMsg {
    Task(Arc<Mutex<usize>>),
}

#[no_mangle]
pub fn rs_do_something_finish(_src: GObject, _result: GAsyncResult, _error: &mut GError) -> usize {
    eprintln!(
        "{:?} !rs_do_something_finish() called",
        std::thread::current()
    );
    // TODO: cast GAsyncResult back to the task and unref it?

    1
}

async fn rs_worker(mut receiver: futures::channel::mpsc::Receiver<SomeMsg>) {
    loop {
        eprintln!("{:?} thread waiting...", std::thread::current());
        while let Some(msg) = receiver.next().await {
            // Perform every task in an available worker thread rather than this one
            tokio::spawn(async move {
                eprintln!("{:?} msg: {:?}", std::thread::current(), msg);
                match msg {
                    SomeMsg::Task(task) => {
                        let task = *task.lock().unwrap() as *mut GTask;
                        unsafe { g_task_return_boolean(task, 1) }
                    }
                }
            });
        }
    }
}

#[no_mangle]
fn rs_init() {
    eprintln!("{:?} enter rs_init()", std::thread::current());

    let (_sender, receiver) = futures::channel::mpsc::channel::<SomeMsg>(10240);

    unsafe { SENDER = Some(_sender) };

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

#[no_mangle]
pub fn rs_do_something_async(callback: GAsyncReadyCallback, user_data: gpointer) {
    let task: Arc<Mutex<usize>>;
    unsafe {
        let obj = g_object_new(G_TYPE_OBJECT, std::ptr::null());
        let can = g_cancellable_new();
        let raw_task = g_task_new(obj, can, callback, user_data) as usize;
        task = Arc::new(Mutex::new(raw_task));
    }

    let new_task = task.clone();

    let sender = unsafe { SENDER.clone() };
    eprintln!("{:?} Sending task down", std::thread::current());
    block_on(sender.unwrap().send(SomeMsg::Task(new_task))).unwrap();
    eprintln!("{:?} Sent task down", std::thread::current());
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
