AC_PREREQ(2.63)
AC_INIT([rust-gtask], 0.1)
AM_SILENT_RULES([yes])
AC_CONFIG_MACRO_DIR([m4])

AC_PROG_CC
AM_PROG_CC_C_O

AM_INIT_AUTOMAKE([-Wno-portability subdir-objects])

PKG_CHECK_MODULES(DEPS,
		  glib-2.0 >= 2.56
		  gobject-2.0 >= 2.56
		  gio-2.0 >= 2.56
		  gio-unix-2.0 >= 2.56
	  )

AC_OUTPUT([Makefile])
AC_OUTPUT([src/Makefile])
