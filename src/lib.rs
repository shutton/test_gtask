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
    eprintln!("Thanks for calling my finish function!");
    // TODO: cast GAsyncResult back to the task and unref it?

    1
}

async fn rs_worker(mut receiver: futures::channel::mpsc::Receiver<SomeMsg>) {
    loop {
        eprintln!("thread waiting...");
        while let Some(msg) = receiver.next().await {
            eprintln!("msg: {:?}", msg);
            match msg {
                SomeMsg::Task(task) => {
                    let task = *task.lock().unwrap() as *mut GTask;
                    unsafe { g_task_return_boolean(task, 1) }
                }
            }
        }
    }
}

#[no_mangle]
fn rs_init() {
    eprintln!("enter rs_init()");

    let (_sender, receiver) = futures::channel::mpsc::channel::<SomeMsg>(10240);

    unsafe { SENDER = Some(_sender) };

    // Spawn a thread to run a Tokio event loop
    thread::spawn(move || {
        use tokio::runtime::Runtime;

        let mut rt = Runtime::new().unwrap();
        rt.block_on(rs_worker(receiver));
    });

    eprintln!("exit rs_init()");
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

    #[cfg(disabled)]
    thread::spawn(move || {
        let new_task = *new_task.lock().unwrap() as *mut GTask;
        unsafe { g_task_return_boolean(new_task, 1) }
    });

    let sender;
    unsafe {
        sender = SENDER.clone();
    };
    block_on(sender.unwrap().send(SomeMsg::Task(new_task))).unwrap();
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
