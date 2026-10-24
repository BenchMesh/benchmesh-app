/* A "window manager" that announces _NET_WM_FRAME_DRAWN and never sends it: what GNOME 42
 * does to a window when it loses a frame (check.sh). Runs until killed. */
#include <X11/Xlib.h>
#include <X11/Xatom.h>
#include <stdio.h>
#include <unistd.h>
int main(void) {
  Display *d = XOpenDisplay(NULL);
  if (!d) { fprintf(stderr, "no display\n"); return 1; }
  Window root = DefaultRootWindow(d);
  Window check = XCreateSimpleWindow(d, root, -10, -10, 1, 1, 0, 0, 0);
  Atom wmcheck = XInternAtom(d, "_NET_SUPPORTING_WM_CHECK", False);
  XChangeProperty(d, root, wmcheck, XA_WINDOW, 32, PropModeReplace, (unsigned char *)&check, 1);
  XChangeProperty(d, check, wmcheck, XA_WINDOW, 32, PropModeReplace, (unsigned char *)&check, 1);
  XChangeProperty(d, check, XInternAtom(d, "_NET_WM_NAME", False), XInternAtom(d, "UTF8_STRING", False), 8,
                  PropModeReplace, (unsigned char *)"fakewm", 6);
  Atom supported[] = { XInternAtom(d, "_NET_WM_FRAME_DRAWN", False), XInternAtom(d, "_NET_WM_FRAME_TIMINGS", False),
                       XInternAtom(d, "_NET_WM_SYNC_REQUEST", False), XInternAtom(d, "_NET_SUPPORTING_WM_CHECK", False) };
  XChangeProperty(d, root, XInternAtom(d, "_NET_SUPPORTED", False), XA_ATOM, 32, PropModeReplace,
                  (unsigned char *)supported, sizeof supported / sizeof supported[0]);
  XSync(d, False);
  printf("fakewm ready\n");
  fflush(stdout);
  for (;;) pause();
}
