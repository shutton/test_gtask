#include <glib.h>
#include <gio/gio.h>

void rs_do_something_async(GAsyncReadyCallback callback, gpointer user_data);
gboolean rs_do_something_finish(GObject *src, GAsyncResult *res, GError **error);

void my_callback(GObject *src, GAsyncResult *res, gpointer user_data)
{
    GMainLoop *mainloop = user_data;
    GError *err = NULL;

    g_printerr("Callback fired!\n");
    gboolean result = rs_do_something_finish(src, res, &err);
    g_printerr("Callback result = %d\n", result);
    g_main_loop_quit(mainloop);
}

int main()
{
    GMainLoop *mainloop = g_main_loop_new(g_main_context_default(), FALSE);
    g_printerr("Calling\n");
    rs_do_something_async(my_callback, mainloop);
    g_main_loop_run(mainloop);
    g_printerr("Called\n");
    return 0;
}
