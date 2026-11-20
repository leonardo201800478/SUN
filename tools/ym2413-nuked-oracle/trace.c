/*
 * YM2413 Nuked-OPLL external oracle trace adapter.
 *
 * This file is project-authored glue. It includes the public opll.h from an
 * independently checked-out Nuked-OPLL tree and links that tree's opll.c;
 * it does not contain or copy Nuked implementation code.
 *
 * Output: one JSON object per OPLL_Clock() invocation, after the call returns.
 */
#include <errno.h>
#include <inttypes.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include "opll.h"

#define MAX_EVENTS 65536

typedef struct {
    uint64_t call_index;
    uint32_t port;
    uint8_t value;
} input_event;

static void print_u32_array(const uint32_t *v, size_t n) {
    putchar('[');
    for (size_t i = 0; i < n; ++i) {
        if (i) putchar(',');
        printf("%" PRIu32, v[i]);
    }
    putchar(']');
}

static void print_u16_array(const uint16_t *v, size_t n) {
    putchar('[');
    for (size_t i = 0; i < n; ++i) {
        if (i) putchar(',');
        printf("%u", (unsigned)v[i]);
    }
    putchar(']');
}

static void print_u8_array(const uint8_t *v, size_t n) {
    putchar('[');
    for (size_t i = 0; i < n; ++i) {
        if (i) putchar(',');
        printf("%u", (unsigned)v[i]);
    }
    putchar(']');
}

static void print_i16_array(const int16_t *v, size_t n) {
    putchar('[');
    for (size_t i = 0; i < n; ++i) {
        if (i) putchar(',');
        printf("%d", (int)v[i]);
    }
    putchar(']');
}

static int load_events(const char *path, input_event *events, size_t *count) {
    FILE *f = fopen(path, "r");
    char line[256];
    *count = 0;
    if (!f) {
        fprintf(stderr, "cannot open events file '%s': %s\n", path, strerror(errno));
        return 0;
    }
    while (fgets(line, sizeof line, f)) {
        unsigned long long clock;
        unsigned port, value;
        char *p = line;
        while (*p == ' ' || *p == '\t') ++p;
        if (*p == '\0' || *p == '\n' || *p == '#') continue;
        if (sscanf(p, "%llu,%u,%u", &clock, &port, &value) != 3 ||
            port > 1 || value > 255 || *count >= MAX_EVENTS) {
            fprintf(stderr, "invalid event row or event limit exceeded: %s", line);
            fclose(f);
            return 0;
        }
        if (*count && clock <= events[*count - 1].call_index) {
            fprintf(stderr, "events must have strictly increasing OPLL_Clock call indices\n");
            fclose(f);
            return 0;
        }
        events[*count].call_index = (uint64_t)clock;
        events[*count].port = port;
        events[*count].value = (uint8_t)value;
        ++*count;
    }
    fclose(f);
    return 1;
}

int main(int argc, char **argv) {
    uint64_t clocks = 0;
    uint32_t chip_type = opll_type_ym2413;
    const char *events_path = NULL;
    input_event *events = calloc(MAX_EVENTS, sizeof(*events));
    size_t event_count = 0, event_index = 0;
    opll_t chip;
    if (!events) return 2;

    for (int i = 1; i < argc; ++i) {
        if (!strcmp(argv[i], "--calls") && i + 1 < argc) {
            clocks = strtoull(argv[++i], NULL, 10);
        } else if (!strcmp(argv[i], "--events") && i + 1 < argc) {
            events_path = argv[++i];
        } else if (!strcmp(argv[i], "--chip-type") && i + 1 < argc) {
            chip_type = (uint32_t)strtoul(argv[++i], NULL, 0);
        } else {
            fprintf(stderr, "usage: %s --calls N [--events file.csv] [--chip-type 0]\n", argv[0]);
            free(events);
            return 2;
        }
    }
    if (clocks == 0) {
        fprintf(stderr, "--calls must be greater than zero\n");
        free(events);
        return 2;
    }
    if (events_path && !load_events(events_path, events, &event_count)) {
        free(events);
        return 2;
    }

    OPLL_Reset(&chip, chip_type);
    puts("{\"schema\":\"ym2413-nuked-opll-trace-v1\",\"time_unit\":\"opll_clock_call\",\"sample_point\":\"after_OPLL_Clock\",\"arrays\":{\"pg_phase\":18,\"eg_state\":18,\"eg_level\":18,\"op_fb1\":9,\"op_fb2\":9}}");

    for (uint64_t tick = 0; tick < clocks; ++tick) {
        int32_t output[2] = {0, 0};
        uint32_t cycle_before = chip.cycles;
        while (event_index < event_count && events[event_index].call_index == tick) {
            OPLL_Write(&chip, events[event_index].port, events[event_index].value);
            ++event_index;
        }
        if (event_index < event_count && events[event_index].call_index < tick) {
            fprintf(stderr, "event call index was skipped\n");
            free(events);
            return 3;
        }

        OPLL_Clock(&chip, output);
        printf("{\"tick\":%" PRIu64 ",\"cycle_before\":%" PRIu32 ",\"cycle_after\":%" PRIu32,
               tick, cycle_before, chip.cycles);
        printf(",\"slot\":%" PRIu32 ",\"output_m\":%" PRId32 ",\"output_r\":%" PRId32,
               chip.slot, output[0], output[1]);
        printf(",\"pg_inc\":%" PRIu32 ",\"pg_phase_next\":%" PRIu32 ",\"pg_phase\":",
               chip.pg_inc, chip.pg_phase_next);
        print_u32_array(chip.pg_phase, 18);
        printf(",\"pg_out\":%u", (unsigned)chip.pg_out);
        printf(",\"eg_counter_state\":%u,\"eg_timer\":%" PRIu32 ",\"eg_timer_carry\":%u",
               (unsigned)chip.eg_counter_state, chip.eg_timer, (unsigned)chip.eg_timer_carry);
        printf(",\"eg_state\":"); print_u8_array(chip.eg_state, 18);
        printf(",\"eg_level\":"); print_u8_array(chip.eg_level, 18);
        printf(",\"eg_out\":%u", (unsigned)chip.eg_out);
        printf(",\"lfo_counter\":%u,\"lfo_vib_counter\":%u,\"lfo_am_counter\":%u,\"lfo_am_out\":%u",
               (unsigned)chip.lfo_counter, (unsigned)chip.lfo_vib_counter,
               (unsigned)chip.lfo_am_counter, (unsigned)chip.lfo_am_out);
        printf(",\"op_fb1\":"); print_i16_array(chip.op_fb1, 9);
        printf(",\"op_fb2\":"); print_i16_array(chip.op_fb2, 9);
        printf(",\"op_fbsum\":%d,\"op_mod\":%d,\"op_neg\":%u,\"op_logsin\":%u,\"op_exp_m\":%u,\"op_exp_s\":%u",
               (int)chip.op_fbsum, (int)chip.op_mod, (unsigned)chip.op_neg,
               (unsigned)chip.op_logsin, (unsigned)chip.op_exp_m, (unsigned)chip.op_exp_s);
        printf(",\"ch_out\":%d,\"ch_out_hh\":%d,\"ch_out_tm\":%d,\"ch_out_bd\":%d,\"ch_out_sd\":%d,\"ch_out_tc\":%d",
               (int)chip.ch_out, (int)chip.ch_out_hh, (int)chip.ch_out_tm,
               (int)chip.ch_out_bd, (int)chip.ch_out_sd, (int)chip.ch_out_tc);
        printf(",\"fnum\":"); print_u16_array(chip.fnum, 9);
        printf(",\"block\":"); print_u8_array(chip.block, 9);
        printf(",\"kon\":"); print_u8_array(chip.kon, 9);
        printf(",\"son\":"); print_u8_array(chip.son, 9);
        printf(",\"rhythm\":%u,\"rm_noise\":%" PRIu32,
               (unsigned)chip.rhythm, chip.rm_noise);
        printf(",\"output_pins_m\":%d,\"output_pins_r\":%d}\n",
               (int)chip.output_m, (int)chip.output_r);
    }

    if (event_index != event_count) {
        fprintf(stderr, "one or more events occur after the requested trace window\n");
        free(events);
        return 4;
    }
    free(events);
    return 0;
}
