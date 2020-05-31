#include <glib.h>
#include <gio/gio.h>

void rs_do_something_async(GAsyncReadyCallback callback, gpointer user_data);
void rs_init();
gsize rs_do_something_finish(GObject *src, GAsyncResult *res, GError **error);

gint tasks_pending = 0;

void my_callback(GObject *src, GAsyncResult *res, gpointer user_data)
{
	GMainLoop *mainloop = user_data;
	GError *err = NULL;

	gsize result = rs_do_something_finish(src, res, &err);
	tasks_pending--;

	if (tasks_pending == 0)
		g_main_loop_quit(mainloop);
}

int main()
{
	GMainLoop *mainloop = g_main_loop_new(g_main_context_default(), FALSE);
	g_printerr("Initializing Rust subsystem");
	rs_init();

	gint i;
	for (i = 0; i < 10; i++) {
		tasks_pending++;
		rs_do_something_async(my_callback, mainloop);
	}
	g_main_loop_run(mainloop);
	return 0;
}
