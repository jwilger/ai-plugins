#define _POSIX_C_SOURCE 200809L

#include <errno.h>
#include <signal.h>
#include <poll.h>
#include <stdbool.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <time.h>
#include <unistd.h>
#include <sys/prctl.h>
#include <wayland-client.h>
#include "wlr-virtual-pointer-unstable-v1-client-protocol.h"

#define MAX_LINE 160
#define MAX_HELD 8

static volatile sig_atomic_t stopping = 0;
static struct wl_display *display;
static struct zwlr_virtual_pointer_manager_v1 *manager;
static struct zwlr_virtual_pointer_v1 *pointer;
static uint32_t held[MAX_HELD];
static size_t held_count;

static void sync_done(void *data, struct wl_callback *callback, uint32_t serial) {
    (void)serial;
    bool *done = data;
    *done = true;
    wl_callback_destroy(callback);
}

static const struct wl_callback_listener sync_listener = {.done = sync_done};

static int bounded_roundtrip(bool during_cleanup) {
    bool done = false;
    struct wl_callback *callback = wl_display_sync(display);
    if (callback == NULL) return -1;
    int status = -1;
    if (wl_callback_add_listener(callback, &sync_listener, &done) != 0)
        goto finished;
    if (wl_display_flush(display) < 0 && errno != EAGAIN) goto finished;
    int remaining_ms = 2000;
    while (!done && (during_cleanup || !stopping) && remaining_ms > 0) {
        struct pollfd descriptor = {.fd = wl_display_get_fd(display), .events = POLLIN};
        struct timespec before, after;
        clock_gettime(CLOCK_MONOTONIC, &before);
        int result = poll(&descriptor, 1, remaining_ms);
        clock_gettime(CLOCK_MONOTONIC, &after);
        int elapsed = (int)((after.tv_sec - before.tv_sec) * 1000 +
                            (after.tv_nsec - before.tv_nsec) / 1000000);
        remaining_ms -= elapsed > 0 ? elapsed : 1;
        if (result < 0 && errno == EINTR) continue;
        if (result <= 0 || !(descriptor.revents & POLLIN) || wl_display_dispatch(display) < 0)
            goto finished;
    }
    status = done && (during_cleanup || !stopping) ? 0 : -1;
finished:
    /* sync_done owns destruction after delivery. All other exits must remove
       the listener before its stack-backed data expires or cleanup dispatches. */
    if (!done) wl_callback_destroy(callback);
    return status;
}

static uint32_t timestamp_ms(void) {
    struct timespec now;
    if (clock_gettime(CLOCK_MONOTONIC, &now) != 0) return 0;
    return (uint32_t)((uint64_t)now.tv_sec * 1000u + (uint64_t)now.tv_nsec / 1000000u);
}

static void signal_handler(int signum) {
    (void)signum;
    stopping = 1;
}

static void registry_global(void *data, struct wl_registry *registry, uint32_t name,
                            const char *interface, uint32_t version) {
    (void)data;
    if (strcmp(interface, zwlr_virtual_pointer_manager_v1_interface.name) == 0) {
        uint32_t selected = version < 2 ? version : 2;
        manager = wl_registry_bind(registry, name,
                                   &zwlr_virtual_pointer_manager_v1_interface,
                                   selected);
    }
}

static void registry_remove(void *data, struct wl_registry *registry, uint32_t name) {
    (void)data; (void)registry; (void)name;
}

static const struct wl_registry_listener registry_listener = {
    .global = registry_global,
    .global_remove = registry_remove,
};

#ifdef POINTER_HELPER_UNIT_TEST
enum test_event_type {
    TEST_MOTION_ABSOLUTE,
    TEST_BUTTON,
    TEST_AXIS_SOURCE,
    TEST_AXIS_DISCRETE,
    TEST_FRAME,
};

struct test_event {
    enum test_event_type type;
    int64_t values[4];
};

static struct test_event test_events[16];
static size_t test_event_count;

static void record_event(enum test_event_type type, int64_t a, int64_t b,
                         int64_t c, int64_t d) {
    if (test_event_count >= sizeof(test_events) / sizeof(test_events[0])) abort();
    test_events[test_event_count++] = (struct test_event){
        .type = type,
        .values = {a, b, c, d},
    };
}

static void emit_motion_absolute(uint32_t x, uint32_t y, uint32_t x_extent,
                                 uint32_t y_extent) {
    record_event(TEST_MOTION_ABSOLUTE, x, y, x_extent, y_extent);
}

static void emit_button(uint32_t button, uint32_t state) {
    record_event(TEST_BUTTON, button, state, 0, 0);
}

static void emit_axis_source(uint32_t source) {
    record_event(TEST_AXIS_SOURCE, source, 0, 0, 0);
}

static void emit_axis_discrete(uint32_t axis, wl_fixed_t value, int32_t discrete) {
    record_event(TEST_AXIS_DISCRETE, axis, value, discrete, 0);
}

static int flush_frame(void) {
    record_event(TEST_FRAME, 0, 0, 0, 0);
    return 0;
}
#else
static void emit_motion_absolute(uint32_t x, uint32_t y, uint32_t x_extent,
                                 uint32_t y_extent) {
    zwlr_virtual_pointer_v1_motion_absolute(pointer, timestamp_ms(), x, y,
                                            x_extent, y_extent);
}

static void emit_button(uint32_t button, uint32_t state) {
    zwlr_virtual_pointer_v1_button(pointer, timestamp_ms(), button, state);
}

static void emit_axis_source(uint32_t source) {
    zwlr_virtual_pointer_v1_axis_source(pointer, source);
}

static void emit_axis_discrete(uint32_t axis, wl_fixed_t value, int32_t discrete) {
    zwlr_virtual_pointer_v1_axis_discrete(pointer, timestamp_ms(), axis, value,
                                          discrete);
}

static int flush_frame(void) {
    zwlr_virtual_pointer_v1_frame(pointer);
    return bounded_roundtrip(false);
}
#endif

static bool next_release(uint32_t *button) {
    if (held_count == 0) return false;
    *button = held[--held_count];
    return true;
}

static void release_all(void) {
    if (pointer == NULL || display == NULL) return;
    uint32_t button;
    while (next_release(&button)) {
        zwlr_virtual_pointer_v1_button(pointer, timestamp_ms(), button,
                                       WL_POINTER_BUTTON_STATE_RELEASED);
        zwlr_virtual_pointer_v1_frame(pointer);
    }
    /* A compositor disconnect makes delivery impossible. Otherwise flush the
       reverse-order releases before destroying the short-lived device. */
    bounded_roundtrip(true);
}

static bool parse_u32(const char *text, uint32_t *value) {
    char *end = NULL;
    errno = 0;
    unsigned long parsed = strtoul(text, &end, 10);
    if (errno || text == end || *end != '\0' || parsed > UINT32_MAX) return false;
    *value = (uint32_t)parsed;
    return true;
}

static bool parse_int(const char *text, int *value) {
    char *end = NULL;
    errno = 0;
    long parsed = strtol(text, &end, 10);
    if (errno || text == end || *end != '\0' || parsed < -20 || parsed > 20) return false;
    *value = (int)parsed;
    return true;
}

static int perform(char *line) {
    char *save = NULL;
    char *kind = strtok_r(line, " ", &save);
    if (kind == NULL) return -1;
    if (strcmp(kind, "move") == 0) {
        uint32_t values[4];
        for (size_t index = 0; index < 4; ++index) {
            char *part = strtok_r(NULL, " ", &save);
            if (part == NULL || !parse_u32(part, &values[index])) return -1;
        }
        if (strtok_r(NULL, " ", &save) != NULL || values[2] == 0 || values[3] == 0 ||
            values[0] > values[2] || values[1] > values[3]) return -1;
        emit_motion_absolute(values[0], values[1], values[2], values[3]);
    } else if (strcmp(kind, "button") == 0) {
        uint32_t position[4], button, state;
        for (size_t index = 0; index < 4; ++index) {
            char *part = strtok_r(NULL, " ", &save);
            if (part == NULL || !parse_u32(part, &position[index])) return -1;
        }
        char *button_text = strtok_r(NULL, " ", &save), *state_text = strtok_r(NULL, " ", &save);
        if (button_text == NULL || state_text == NULL || strtok_r(NULL, " ", &save) != NULL ||
            !parse_u32(button_text, &button) || !parse_u32(state_text, &state) || state > 1 ||
            (button != 0x110 && button != 0x111 && button != 0x112) ||
            position[2] == 0 || position[3] == 0 || position[0] > position[2] || position[1] > position[3]) return -1;
        size_t found = held_count;
        for (size_t index = 0; index < held_count; ++index) if (held[index] == button) found = index;
        if (state == WL_POINTER_BUTTON_STATE_PRESSED) {
            if (found != held_count || held_count == MAX_HELD) return -1;
            held[held_count++] = button;
        } else {
            if (found == held_count) return -1;
        }
        emit_motion_absolute(position[0], position[1], position[2], position[3]);
        emit_button(button, state);
        if (flush_frame() != 0) return -1;
        if (state == WL_POINTER_BUTTON_STATE_RELEASED) {
            memmove(&held[found], &held[found + 1], (held_count - found - 1) * sizeof(held[0]));
            --held_count;
        }
        return 0;
    } else if (strcmp(kind, "scroll") == 0) {
        uint32_t position[4];
        for (size_t index = 0; index < 4; ++index) {
            char *part = strtok_r(NULL, " ", &save);
            if (part == NULL || !parse_u32(part, &position[index])) return -1;
        }
        int dx, dy;
        char *dx_text = strtok_r(NULL, " ", &save), *dy_text = strtok_r(NULL, " ", &save);
        if (dx_text == NULL || dy_text == NULL || strtok_r(NULL, " ", &save) != NULL ||
            !parse_int(dx_text, &dx) || !parse_int(dy_text, &dy) || (dx == 0 && dy == 0) ||
            position[2] == 0 || position[3] == 0 || position[0] > position[2] || position[1] > position[3]) return -1;
        emit_motion_absolute(position[0], position[1], position[2], position[3]);
        emit_axis_source(WL_POINTER_AXIS_SOURCE_WHEEL);
        if (dx != 0) {
            emit_axis_discrete(WL_POINTER_AXIS_HORIZONTAL_SCROLL,
                               wl_fixed_from_int(dx * 15), dx);
        }
        if (dy != 0) {
            emit_axis_discrete(WL_POINTER_AXIS_VERTICAL_SCROLL,
                               wl_fixed_from_int(dy * 15), dy);
        }
    } else {
        return -1;
    }
    return flush_frame();
}

static int setup(void) {
    display = wl_display_connect(NULL);
    if (display == NULL) return -1;
    struct wl_registry *registry = wl_display_get_registry(display);
    if (registry == NULL) return -1;
    if (wl_registry_add_listener(registry, &registry_listener, NULL) != 0 ||
        bounded_roundtrip(false) < 0 || manager == NULL) {
        wl_registry_destroy(registry);
        return -1;
    }
    pointer = zwlr_virtual_pointer_manager_v1_create_virtual_pointer(manager, NULL);
    wl_registry_destroy(registry);
    return pointer == NULL || bounded_roundtrip(false) < 0 ? -1 : 0;
}

static void cleanup(void) {
    release_all();
    if (pointer != NULL) zwlr_virtual_pointer_v1_destroy(pointer);
    if (manager != NULL) zwlr_virtual_pointer_manager_v1_destroy(manager);
    if (display != NULL) wl_display_disconnect(display);
}

#ifndef POINTER_HELPER_UNIT_TEST
int main(void) {
    struct sigaction action = {0};
    action.sa_handler = signal_handler;
    sigemptyset(&action.sa_mask);
    sigaction(SIGINT, &action, NULL);
    sigaction(SIGTERM, &action, NULL);
    sigaction(SIGHUP, &action, NULL);
    sigaction(SIGALRM, &action, NULL);
    signal(SIGPIPE, SIG_IGN);

    pid_t parent = getppid();
    if (prctl(PR_SET_PDEATHSIG, SIGTERM) != 0 || getppid() != parent)
        return 1;
    alarm(12);

    if (setup() != 0) {
        cleanup();
        return 1;
    }
    puts("ready");
    fflush(stdout);

    char line[MAX_LINE];
    int status = 1;
    while (!stopping) {
        errno = 0;
        if (fgets(line, sizeof(line), stdin) == NULL) break;
        if (stopping) break;
        size_t length = strlen(line);
        if (length == 0 || line[length - 1] != '\n') break;
        line[length - 1] = '\0';
        if (strcmp(line, "finish") == 0) {
            if (held_count != 0) break;
            puts("done");
            fflush(stdout);
            status = 0;
            break;
        }
        if (perform(line) != 0) break;
        puts("ok");
        fflush(stdout);
    }
    cleanup();
    return status;
}
#else
int main(void) {
    held[0] = 0x110;
    held[1] = 0x111;
    held[2] = 0x112;
    held_count = 3;
    uint32_t button;
    if (!next_release(&button) || button != 0x112) return 1;
    if (!next_release(&button) || button != 0x111) return 1;
    if (!next_release(&button) || button != 0x110) return 1;
    if (next_release(&button)) return 1;

    char vertical_scroll[] = "scroll 40 70 1000 1200 0 2";
    test_event_count = 0;
    if (perform(vertical_scroll) != 0 || test_event_count != 4) return 1;
    if (test_events[0].type != TEST_MOTION_ABSOLUTE ||
        test_events[0].values[0] != 40 || test_events[0].values[1] != 70 ||
        test_events[0].values[2] != 1000 || test_events[0].values[3] != 1200)
        return 1;
    if (test_events[1].type != TEST_AXIS_SOURCE ||
        test_events[1].values[0] != WL_POINTER_AXIS_SOURCE_WHEEL)
        return 1;
    if (test_events[2].type != TEST_AXIS_DISCRETE ||
        test_events[2].values[0] != WL_POINTER_AXIS_VERTICAL_SCROLL ||
        test_events[2].values[1] != wl_fixed_from_int(30) ||
        test_events[2].values[2] != 2)
        return 1;
    if (test_events[3].type != TEST_FRAME) return 1;

    /* Match the pinned Hyprland accumulator: axis_discrete supplies 120 units
       per wheel step and frame emits it. A same-frame axis_stop would replace
       these values with zero, which is the regression this sequence excludes. */
    int32_t compositor_delta_discrete = 0;
    for (size_t index = 0; index < test_event_count; ++index) {
        if (test_events[index].type == TEST_AXIS_DISCRETE)
            compositor_delta_discrete = (int32_t)test_events[index].values[2] * 120;
        if (test_events[index].type == TEST_FRAME && compositor_delta_discrete != 240)
            return 1;
    }

    char diagonal_scroll[] = "scroll 4 7 100 120 -2 3";
    test_event_count = 0;
    if (perform(diagonal_scroll) != 0 || test_event_count != 5) return 1;
    if (test_events[0].type != TEST_MOTION_ABSOLUTE ||
        test_events[1].type != TEST_AXIS_SOURCE ||
        test_events[2].type != TEST_AXIS_DISCRETE ||
        test_events[2].values[0] != WL_POINTER_AXIS_HORIZONTAL_SCROLL ||
        test_events[2].values[2] != -2 ||
        test_events[3].type != TEST_AXIS_DISCRETE ||
        test_events[3].values[0] != WL_POINTER_AXIS_VERTICAL_SCROLL ||
        test_events[3].values[2] != 3 ||
        test_events[4].type != TEST_FRAME)
        return 1;

    signal_handler(SIGTERM);
    return stopping == 1 ? 0 : 1;
}
#endif
