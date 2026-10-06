#define _POSIX_C_SOURCE 200809L
#include <errno.h>
#include <poll.h>
#include <stdbool.h>
#include <stdio.h>
#include <wayland-client.h>

struct fake_callback {
    bool destroyed;
    const struct wl_callback_listener *listener;
    void *data;
};
static struct fake_callback callbacks[4];
static size_t callback_count;
static bool listener_fails, flush_fails, dispatch_fails, timeout_poll;

static struct wl_callback *fake_sync(struct wl_display *unused) {
    (void)unused;
    if (callback_count == 4) return NULL;
    struct fake_callback *callback = &callbacks[callback_count++];
    *callback = (struct fake_callback){0};
    return (struct wl_callback *)callback;
}
static __attribute__((noinline)) int fake_listener(struct wl_callback *callback,
                         const struct wl_callback_listener *listener, void *data) {
    struct fake_callback *fake = (struct fake_callback *)callback;
    fake->listener = listener;
    fake->data = data;
    return listener_fails ? -1 : 0;
}
static void fake_destroy(struct wl_callback *callback) {
    struct fake_callback *fake = (struct fake_callback *)callback;
    fake->destroyed = true;
    fake->listener = NULL;
    fake->data = NULL;
}
static int fake_flush(struct wl_display *unused) {
    (void)unused;
    if (flush_fails) { errno = EPIPE; return -1; }
    return 0;
}
static int fake_fd(struct wl_display *unused) { (void)unused; return 1; }
static int fake_poll(struct pollfd *descriptors, nfds_t count, int timeout) {
    (void)count; (void)timeout;
    if (timeout_poll) return 0;
    descriptors[0].revents = POLLIN;
    return 1;
}
static int fake_dispatch(struct wl_display *unused) {
    (void)unused;
    if (dispatch_fails) return -1;
    for (size_t index = 0; index < callback_count; ++index) {
        struct fake_callback *callback = &callbacks[index];
        if (!callback->destroyed && callback->listener)
            callback->listener->done(callback->data, (struct wl_callback *)callback, 0);
    }
    return 0;
}

/* Replace only the sync boundary; no compositor connection or input is made. */
#define wl_display_sync fake_sync
#define wl_callback_add_listener fake_listener
#define wl_callback_destroy fake_destroy
#define wl_display_flush fake_flush
#define wl_display_get_fd fake_fd
#define poll fake_poll
#define wl_display_dispatch fake_dispatch
#define main original_pointer_tests
#include "../src/hyprland-pointer-adapter-helper.c"
#undef main

static int require_released(const char *scenario) {
    if (callback_count && !callbacks[callback_count - 1].destroyed) {
        fprintf(stderr, "%s retained a callback into expired stack storage\n", scenario);
        return 1;
    }
    return 0;
}

int main(void) {
    timeout_poll = true;
    if (bounded_roundtrip(false) != -1 || require_released("timeout")) return 1;
    timeout_poll = false;
    listener_fails = true;
    if (bounded_roundtrip(false) != -1 || require_released("listener failure")) return 1;
    listener_fails = false;
    flush_fails = true;
    if (bounded_roundtrip(false) != -1 || require_released("flush failure")) return 1;
    flush_fails = false;
    dispatch_fails = true;
    if (bounded_roundtrip(false) != -1 || require_released("dispatch failure")) return 1;
    dispatch_fails = false;
    callback_count = 0;
    stopping = 1;
    if (bounded_roundtrip(false) != -1 || require_released("cancellation")) return 1;
    /* Cleanup may dispatch a later sync; cancelled callbacks must be absent. */
    if (bounded_roundtrip(true) != 0 || require_released("cleanup")) return 1;
    return 0;
}
