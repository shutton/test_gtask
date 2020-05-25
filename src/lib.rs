use gio_sys::{
    g_cancellable_new, g_task_new, g_task_return_boolean, GAsyncReadyCallback, GAsyncResult,
};
// use glib::Object;
use gio_sys::GTask;
use glib_sys::{gpointer, GError};
use gobject_sys::{g_object_new, GObject, G_TYPE_OBJECT};
use std::sync::{Arc, Mutex};
use std::thread;

#[no_mangle]
#[allow(unused)]
pub fn rs_do_something_finish(src: GObject, result: GAsyncResult, error: &mut GError) -> usize {
    eprintln!("Thanks for calling my finish function!");

    1
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
    thread::spawn(move || {
        let new_task = *new_task.lock().unwrap() as *mut GTask;
        unsafe { g_task_return_boolean(new_task, 1) }
    });
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
