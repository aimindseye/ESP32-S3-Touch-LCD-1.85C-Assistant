#include "esp_heap_caps.h"
#include <ctype.h>
#include <stdbool.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <stdatomic.h>

#include "freertos/FreeRTOS.h"
#include "freertos/task.h"
#include "freertos/stream_buffer.h"
#include "esp_err.h"
#include "esp_http_client.h"
#include "esp_crt_bundle.h"
#include "mp3dec.h"

extern bool st77916_audio_pcm_init(uint32_t sample_rate, uint16_t bits_per_sample, uint16_t channels);
extern int32_t st77916_audio_pcm_write(const uint8_t *data, uint32_t len, uint32_t timeout_ms);
extern void st77916_audio_pcm_stop(void);

#define RADIO_INPUT_BYTES 262144
#define RADIO_R10_PREFILL_TARGET_BYTES 196608
#define RADIO_R10_LOW_WATER_BYTES 65536
#define RADIO_R10_RUNTIME_TARGET_BYTES 98304
#define RADIO_R10_PREFILL_READ_CHUNK 4096
#define RADIO_R10_RUNTIME_READ_CHUNK 2048
#define RADIO_R10_MIN_DECODE_BYTES 8192
#define RADIO_R10_PREFILL_SPINS 1024
#define RADIO_R10_RUNTIME_SPINS 64
#define RADIO_SMALL_RESPONSE_BYTES 8192
#define RADIO_RESOLVED_URL_BYTES 512
#define RADIO_HELIX_OUT_SAMPLES (MAX_NCHAN * MAX_NGRAN * MAX_NSAMP)
#define RADIO_MAX_DECODE_ERRORS 96

#define RADIO_R11_STREAM_BYTES 196608
#define RADIO_R11_STREAM_FALLBACK_BYTES 32768
#define RADIO_R11_PREFILL_BYTES 65536
#define RADIO_R11_MIN_START_BYTES 24576
#define RADIO_R11_DECODE_INPUT_BYTES 65536
#define RADIO_R11_DECODE_TARGET_BYTES 32768
#define RADIO_R11_DECODE_LOW_WATER_BYTES 8192
#define RADIO_R11_MIN_DECODE_BYTES 4096
#define RADIO_R11_HTTP_CHUNK_BYTES 2048
#define RADIO_R11_STREAM_SEND_WAIT_MS 100
#define RADIO_R11_STREAM_RECV_WAIT_MS 20
#define RADIO_R11_PREFILL_WAIT_MS 10000
#define RADIO_R11_PRODUCER_STACK_BYTES 6144
#define RADIO_R11_PRODUCER_PRIORITY (tskIDLE_PRIORITY + 2)






enum {
    RADIO_STATUS_IDLE = 0,
    RADIO_STATUS_CONNECTING = 1,
    RADIO_STATUS_BUFFERING = 2,
    RADIO_STATUS_PLAYING = 3,
    RADIO_STATUS_STOPPING = 4,
    RADIO_STATUS_STOPPED = 5,
    RADIO_STATUS_ERROR = 6,
};

static atomic_bool s_radio_stop_requested = ATOMIC_VAR_INIT(false);
static atomic_uint s_radio_volume_percent = ATOMIC_VAR_INIT(60);
static atomic_uint s_radio_elapsed_seconds = ATOMIC_VAR_INIT(0);
static atomic_uint s_radio_buffered_bytes = ATOMIC_VAR_INIT(0);
static atomic_uint s_radio_status = ATOMIC_VAR_INIT(RADIO_STATUS_IDLE);
static atomic_int s_radio_r19_resolve_depth = ATOMIC_VAR_INIT(0);

static inline bool radio_stop_requested(void) {
    return atomic_load_explicit(&s_radio_stop_requested, memory_order_acquire);
}

static inline uint32_t radio_volume_percent(void) {
    return atomic_load_explicit(&s_radio_volume_percent, memory_order_acquire);
}

static inline void radio_set_status(uint32_t status) {
    atomic_store_explicit(&s_radio_status, status, memory_order_release);
}

static inline void radio_set_buffered(uint32_t bytes) {
    atomic_store_explicit(&s_radio_buffered_bytes, bytes, memory_order_release);
}

void st77916_radio_http_mp3_set_volume(uint32_t volume_percent) {
    atomic_store_explicit(&s_radio_volume_percent, volume_percent > 100 ? 100 : volume_percent, memory_order_release);
}

uint32_t st77916_radio_http_mp3_elapsed_seconds(void) {
    return atomic_load_explicit(&s_radio_elapsed_seconds, memory_order_acquire);
}

uint32_t st77916_radio_http_mp3_buffered_bytes(void) {
    return atomic_load_explicit(&s_radio_buffered_bytes, memory_order_acquire);
}

uint32_t st77916_radio_http_mp3_status_code(void) {
    return atomic_load_explicit(&s_radio_status, memory_order_acquire);
}

void st77916_radio_http_mp3_stop_request(void) {
    atomic_store_explicit(&s_radio_stop_requested, true, memory_order_release);
    radio_set_status(RADIO_STATUS_STOPPING);
    printf("radio-r36: stop_request=DEFERRED_TO_STREAM_THREAD i2s_stop_owner=RADIO_THREAD audio=PCM5101_I2S\n");
}

static const char *radio_stream_format_label(const char *url) {
    if (url != NULL && strncmp(url, "https://", 8) == 0) {
        return "HTTPS_MP3";
    }
    return "HTTP_MP3";
}

static void radio_r20_yield(void) {
    vTaskDelay(pdMS_TO_TICKS(1));
}

static const char *radio_r20_url_kind(const char *url) {
    if (url == NULL) return "UNKNOWN";
    if (strstr(url, "/player/") != NULL) return "PLAYER_PAGE";
    if (strstr(url, ".m3u8") != NULL || strstr(url, "hlsp") != NULL || strstr(url, "hls") != NULL) return "HLS_PLAYLIST";
    if (strstr(url, ".m3u") != NULL || strstr(url, "tunein-station") != NULL) return "M3U_PLAYLIST";
    if (strncmp(url, "https://", 8) == 0) return "HTTPS_DIRECT_CANDIDATE";
    if (strncmp(url, "http://", 7) == 0) return "HTTP_DIRECT_CANDIDATE";
    return "UNKNOWN";
}

int32_t st77916_radio_http_mp3_play(const char *url, const char *station_name, uint32_t volume_percent);

static bool radio_r19_url_delim(char c) {
    return c == '\0' || c == '\r' || c == '\n' || c == '"' || c == '\'' ||
           c == '<' || c == '>' || c == ')' || c == '(' || isspace((unsigned char)c);
}

static bool radio_r19_copy_url_at(const char *p, char *out, size_t out_len, const char *original_url) {
    if (p == NULL || out == NULL || out_len < 16) return false;
    size_t n = 0;
    while (p[n] != '\0' && !radio_r19_url_delim(p[n]) && n + 1 < out_len) {
        out[n] = p[n];
        n++;
    }
    while (n > 0 && (out[n - 1] == ',' || out[n - 1] == ';' || out[n - 1] == ']')) n--;
    out[n] = '\0';
    if (n < 10) return false;
    if (original_url != NULL && strcmp(out, original_url) == 0) return false;
    return strncmp(out, "http://", 7) == 0 || strncmp(out, "https://", 8) == 0;
}

static bool radio_r19_extract_first_url(const char *body, char *out, size_t out_len, const char *original_url) {
    if (body == NULL || out == NULL) return false;
    const char *p = body;
    while ((p = strstr(p, "http://")) != NULL) {
        if (radio_r19_copy_url_at(p, out, out_len, original_url)) return true;
        p += 7;
    }
    p = body;
    while ((p = strstr(p, "https://")) != NULL) {
        if (radio_r19_copy_url_at(p, out, out_len, original_url)) return true;
        p += 8;
    }
    return false;
}

static bool radio_r19_looks_like_mp3(const unsigned char *buf, int len) {
    if (buf == NULL || len < 3) return false;
    if (buf[0] == 'I' && buf[1] == 'D' && buf[2] == '3') return true;
    int scan = len < 512 ? len - 1 : 511;
    for (int i = 0; i < scan; i++) {
        if (buf[i] == 0xFF && (buf[i + 1] & 0xE0) == 0xE0) return true;
    }
    return false;
}

static int32_t radio_r19_handle_small_response(esp_http_client_handle_t client,
                                               const char *url,
                                               const char *station_name,
                                               uint32_t volume_percent,
                                               int content_length,
                                               int status_code,
                                               int *handled) {
    *handled = 0;
    if (content_length <= 0 || content_length > RADIO_SMALL_RESPONSE_BYTES || client == NULL) {
        return 0;
    }

    *handled = 1;
    char body[RADIO_SMALL_RESPONSE_BYTES + 1];
    int total = 0;
    while (total < content_length && total < RADIO_SMALL_RESPONSE_BYTES && !radio_stop_requested()) {
        int want = content_length - total;
        if (want > 512) want = 512;
        int n = esp_http_client_read(client, body + total, want);
        if (n <= 0) break;
        total += n;
        radio_r20_yield();
    }
    body[total] = '\0';

    if (radio_r19_looks_like_mp3((const unsigned char *)body, total)) {
        printf("radio-r36-r19: status=SMALL_MP3_RESPONSE station=%s bytes=%d action=FALL_THROUGH_TO_DECODER audio=PCM5101_I2S\n", station_name, total);
        return 0;
    }

    char resolved[RADIO_RESOLVED_URL_BYTES];
    if (radio_r19_extract_first_url(body, resolved, sizeof(resolved), url)) {
        printf("radio-r36-r19: status=PLAYLIST_RESOLVED station=%s status_code=%d bytes=%d url=%s audio=PCM5101_I2S\n", station_name, status_code, total, resolved);
        esp_http_client_close(client);
        esp_http_client_cleanup(client);
        if (atomic_load_explicit(&s_radio_r19_resolve_depth, memory_order_acquire) >= 1) {
            printf("radio-r36-r19: status=PLAYLIST_RECURSION_BLOCKED station=%s audio=PCM5101_I2S\n", station_name);
            return -18;
        }
        atomic_fetch_add_explicit(&s_radio_r19_resolve_depth, 1, memory_order_acq_rel);
        int32_t code = st77916_radio_http_mp3_play(resolved, station_name, volume_percent);
        atomic_fetch_sub_explicit(&s_radio_r19_resolve_depth, 1, memory_order_acq_rel);
        return code;
    }

    printf("radio-r36-r20: status=UNSUPPORTED_URL_KIND station=%s kind=%s status_code=%d bytes=%d content_length=%d action=STOP_NO_DECODE audio=PCM5101_I2S\n",
           station_name, radio_r20_url_kind(url), status_code, total, content_length);
    esp_http_client_close(client);
    esp_http_client_cleanup(client);
    return -19;
}

static void scale_pcm16(short *samples, int sample_count, uint32_t volume_percent) {
    int32_t volume = (int32_t)(volume_percent > 100 ? 100 : volume_percent);
    for (int i = 0; i < sample_count; i++) {
        int32_t scaled = ((int32_t)samples[i] * volume) / 100;
        if (scaled > 32767) scaled = 32767;
        else if (scaled < -32768) scaled = -32768;
        samples[i] = (short)scaled;
    }
}


static int radio_r10_r2_prepare_i2s_stereo(short *samples, int output_samps, int source_channels) {
    if (samples == NULL || output_samps <= 0) {
        return 0;
    }
    if (source_channels == 2) {
        return output_samps;
    }
    if (source_channels == 1) {
        if (output_samps * 2 > RADIO_HELIX_OUT_SAMPLES) {
            return output_samps;
        }
        for (int i = output_samps - 1; i >= 0; i--) {
            short s = samples[i];
            samples[(i * 2)] = s;
            samples[(i * 2) + 1] = s;
        }
        return output_samps * 2;
    }
    return output_samps;
}

static bool compact_input(unsigned char *input, unsigned char **read_ptr, int *bytes_left) {
    if (*read_ptr != input && *bytes_left > 0) {
        memmove(input, *read_ptr, (size_t)*bytes_left);
    }
    *read_ptr = input;
    return true;
}


typedef struct {
    esp_http_client_handle_t client;
    StreamBufferHandle_t stream;
    TaskHandle_t task;
    uint8_t *stream_storage;
    StaticStreamBuffer_t *stream_static;
    uint32_t stream_bytes;
    bool stream_psram;
    atomic_bool stop;
    atomic_bool done;
    atomic_int result;
    atomic_uint total_read;
    atomic_uint total_sent;
} radio_r11_producer_ctx_t;

static uint32_t radio_r11_stream_available(radio_r11_producer_ctx_t *ctx) {
    if (ctx == NULL || ctx->stream == NULL) return 0;
    return (uint32_t)xStreamBufferBytesAvailable(ctx->stream);
}

static void radio_r11_set_buffered_total(radio_r11_producer_ctx_t *ctx, int decode_bytes) {
    uint32_t stream_bytes = radio_r11_stream_available(ctx);
    if (decode_bytes < 0) decode_bytes = 0;
    radio_set_buffered(stream_bytes + (uint32_t)decode_bytes);
}

static void radio_r11_http_producer_task(void *arg) {
    radio_r11_producer_ctx_t *ctx = (radio_r11_producer_ctx_t *)arg;
    uint8_t *chunk = (uint8_t *)malloc(RADIO_R11_HTTP_CHUNK_BYTES);
    if (ctx == NULL || chunk == NULL) {
        if (ctx != NULL) {
            atomic_store_explicit(&ctx->result, -1, memory_order_release);
            atomic_store_explicit(&ctx->done, true, memory_order_release);
        }
        if (chunk != NULL) free(chunk);
        vTaskDelete(NULL);
        return;
    }

    printf("radio-r36-r32: producer=START task=radio_prod stream_bytes=%u chunk=%d psram=%s blocking=YES spin=NO audio=PCM5101_I2S\n",
           (unsigned)ctx->stream_bytes, RADIO_R11_HTTP_CHUNK_BYTES, ctx->stream_psram ? "YES" : "NO");

    int zero_reads = 0;
    while (!radio_stop_requested() && !atomic_load_explicit(&ctx->stop, memory_order_acquire)) {
        int n = esp_http_client_read(ctx->client, (char *)chunk, RADIO_R11_HTTP_CHUNK_BYTES);
        if (n > 0) {
            zero_reads = 0;
            atomic_fetch_add_explicit(&ctx->total_read, (unsigned)n, memory_order_acq_rel);
            int sent_total = 0;
            while (sent_total < n && !radio_stop_requested() && !atomic_load_explicit(&ctx->stop, memory_order_acquire)) {
                size_t sent = xStreamBufferSend(ctx->stream,
                                                chunk + sent_total,
                                                (size_t)(n - sent_total),
                                                pdMS_TO_TICKS(RADIO_R11_STREAM_SEND_WAIT_MS));
                if (sent > 0) {
                    sent_total += (int)sent;
                    atomic_fetch_add_explicit(&ctx->total_sent, (unsigned)sent, memory_order_acq_rel);
                    radio_set_buffered((uint32_t)xStreamBufferBytesAvailable(ctx->stream));
                } else {
                    vTaskDelay(pdMS_TO_TICKS(5));
                }
            }
        } else if (n == 0) {
            zero_reads++;
            vTaskDelay(pdMS_TO_TICKS(5));
            if ((zero_reads % 200) == 0) {
                printf("radio-r36-r31: producer=WAIT_DATA zero_reads=%d buffered=%u audio=PCM5101_I2S\n",
                       zero_reads, (unsigned)radio_r11_stream_available(ctx));
            }
        } else {
            atomic_store_explicit(&ctx->result, n, memory_order_release);
            printf("radio-r36-r31: producer=READ_ERROR err=%d read=%u sent=%u audio=PCM5101_I2S\n",
                   n,
                   atomic_load_explicit(&ctx->total_read, memory_order_acquire),
                   atomic_load_explicit(&ctx->total_sent, memory_order_acquire));
            break;
        }
        vTaskDelay(pdMS_TO_TICKS(1));
    }

    free(chunk);
    atomic_store_explicit(&ctx->done, true, memory_order_release);
    printf("radio-r36-r31: producer=DONE read=%u sent=%u buffered=%u stop=%s audio=PCM5101_I2S\n",
           atomic_load_explicit(&ctx->total_read, memory_order_acquire),
           atomic_load_explicit(&ctx->total_sent, memory_order_acquire),
           (unsigned)radio_r11_stream_available(ctx),
           radio_stop_requested() || atomic_load_explicit(&ctx->stop, memory_order_acquire) ? "YES" : "NO");
    vTaskDelete(NULL);
}

static bool radio_r11_start_producer(radio_r11_producer_ctx_t *ctx, esp_http_client_handle_t client) {
    if (ctx == NULL || client == NULL) return false;
    memset(ctx, 0, sizeof(*ctx));
    ctx->client = client;
    ctx->stream_bytes = RADIO_R11_STREAM_BYTES;
    ctx->stream_psram = false;
    atomic_init(&ctx->stop, false);
    atomic_init(&ctx->done, false);
    atomic_init(&ctx->result, 0);
    atomic_init(&ctx->total_read, 0);
    atomic_init(&ctx->total_sent, 0);

#if (configSUPPORT_STATIC_ALLOCATION == 1)
    ctx->stream_storage = (uint8_t *)heap_caps_malloc((size_t)ctx->stream_bytes + 1, MALLOC_CAP_SPIRAM | MALLOC_CAP_8BIT);
    ctx->stream_static = (StaticStreamBuffer_t *)calloc(1, sizeof(StaticStreamBuffer_t));
    if (ctx->stream_storage != NULL && ctx->stream_static != NULL) {
        ctx->stream = xStreamBufferCreateStatic((size_t)ctx->stream_bytes,
                                                1,
                                                ctx->stream_storage,
                                                ctx->stream_static);
        ctx->stream_psram = (ctx->stream != NULL);
    }
#endif

    if (ctx->stream == NULL) {
        if (ctx->stream_storage != NULL) {
            free(ctx->stream_storage);
            ctx->stream_storage = NULL;
        }
        if (ctx->stream_static != NULL) {
            free(ctx->stream_static);
            ctx->stream_static = NULL;
        }
        ctx->stream_bytes = RADIO_R11_STREAM_FALLBACK_BYTES;
        ctx->stream = xStreamBufferCreate((size_t)ctx->stream_bytes, 1);
        ctx->stream_psram = false;
    }

    if (ctx->stream == NULL) {
        printf("radio-r36-r32: producer=ALLOC_FAILED stream_bytes=%u internal_free=%u psram_free=%u audio=PCM5101_I2S\n",
               (unsigned)ctx->stream_bytes,
               (unsigned)heap_caps_get_free_size(MALLOC_CAP_INTERNAL | MALLOC_CAP_8BIT),
               (unsigned)heap_caps_get_free_size(MALLOC_CAP_SPIRAM | MALLOC_CAP_8BIT));
        return false;
    }

    printf("radio-r36-r32: producer=START_CONFIG stream_bytes=%u psram=%s stack=%d internal_free=%u psram_free=%u audio=PCM5101_I2S\n",
           (unsigned)ctx->stream_bytes,
           ctx->stream_psram ? "YES" : "NO",
           RADIO_R11_PRODUCER_STACK_BYTES,
           (unsigned)heap_caps_get_free_size(MALLOC_CAP_INTERNAL | MALLOC_CAP_8BIT),
           (unsigned)heap_caps_get_free_size(MALLOC_CAP_SPIRAM | MALLOC_CAP_8BIT));

    BaseType_t ok = xTaskCreate(radio_r11_http_producer_task,
                                "radio_prod",
                                RADIO_R11_PRODUCER_STACK_BYTES,
                                ctx,
                                RADIO_R11_PRODUCER_PRIORITY,
                                &ctx->task);
    if (ok != pdPASS || ctx->task == NULL) {
        printf("radio-r36-r32: producer=TASK_CREATE_FAILED ok=%d stream_bytes=%u psram=%s internal_free=%u psram_free=%u audio=PCM5101_I2S\n",
               (int)ok,
               (unsigned)ctx->stream_bytes,
               ctx->stream_psram ? "YES" : "NO",
               (unsigned)heap_caps_get_free_size(MALLOC_CAP_INTERNAL | MALLOC_CAP_8BIT),
               (unsigned)heap_caps_get_free_size(MALLOC_CAP_SPIRAM | MALLOC_CAP_8BIT));
        vStreamBufferDelete(ctx->stream);
        ctx->stream = NULL;
        if (ctx->stream_storage != NULL) {
            free(ctx->stream_storage);
            ctx->stream_storage = NULL;
        }
        if (ctx->stream_static != NULL) {
            free(ctx->stream_static);
            ctx->stream_static = NULL;
        }
        return false;
    }
    return true;
}

static void radio_r11_stop_producer(radio_r11_producer_ctx_t *ctx) {
    if (ctx == NULL) return;
    atomic_store_explicit(&ctx->stop, true, memory_order_release);
    int waited_ms = 0;
    while (ctx->task != NULL && !atomic_load_explicit(&ctx->done, memory_order_acquire) && waited_ms < 4000) {
        vTaskDelay(pdMS_TO_TICKS(10));
        waited_ms += 10;
    }
    if (ctx->task != NULL && !atomic_load_explicit(&ctx->done, memory_order_acquire)) {
        printf("radio-r36-r32: producer=FORCE_DELETE waited_ms=%d audio=PCM5101_I2S\n", waited_ms);
        vTaskDelete(ctx->task);
        ctx->task = NULL;
        atomic_store_explicit(&ctx->done, true, memory_order_release);
    }
    if (ctx->stream != NULL) {
        vStreamBufferDelete(ctx->stream);
        ctx->stream = NULL;
    }
    if (ctx->stream_storage != NULL) {
        free(ctx->stream_storage);
        ctx->stream_storage = NULL;
    }
    if (ctx->stream_static != NULL) {
        free(ctx->stream_static);
        ctx->stream_static = NULL;
    }
}


static bool radio_r11_wait_prefill(radio_r11_producer_ctx_t *ctx, int target_bytes, int timeout_ms) {
    int waited_ms = 0;
    while (!radio_stop_requested() && ctx != NULL && ctx->stream != NULL && waited_ms < timeout_ms) {
        uint32_t available = radio_r11_stream_available(ctx);
        radio_set_buffered(available);
        if ((int)available >= target_bytes) return true;
        if (atomic_load_explicit(&ctx->done, memory_order_acquire) && available >= RADIO_R11_MIN_START_BYTES) return true;
        vTaskDelay(pdMS_TO_TICKS(20));
        waited_ms += 20;
    }
    return ctx != NULL && radio_r11_stream_available(ctx) >= RADIO_R11_MIN_START_BYTES;
}

static bool radio_r11_refill_decode_input(radio_r11_producer_ctx_t *ctx,
                                          unsigned char *input,
                                          unsigned char **read_ptr,
                                          int *bytes_left,
                                          int target_bytes,
                                          int timeout_ms) {
    if (ctx == NULL || ctx->stream == NULL || input == NULL || read_ptr == NULL || bytes_left == NULL) return false;
    compact_input(input, read_ptr, bytes_left);
    if (target_bytes > RADIO_R11_DECODE_INPUT_BYTES) target_bytes = RADIO_R11_DECODE_INPUT_BYTES;

    int no_progress = 0;
    while (!radio_stop_requested() && *bytes_left < target_bytes) {
        int space = RADIO_R11_DECODE_INPUT_BYTES - *bytes_left;
        if (space <= 0) break;
        int want = target_bytes - *bytes_left;
        if (want > space) want = space;
        if (want > 4096) want = 4096;
        size_t got = xStreamBufferReceive(ctx->stream,
                                           input + *bytes_left,
                                           (size_t)want,
                                           pdMS_TO_TICKS(timeout_ms));
        if (got > 0) {
            *bytes_left += (int)got;
            no_progress = 0;
            radio_r11_set_buffered_total(ctx, *bytes_left);
            continue;
        }
        no_progress++;
        radio_r11_set_buffered_total(ctx, *bytes_left);
        if (atomic_load_explicit(&ctx->done, memory_order_acquire) && radio_r11_stream_available(ctx) == 0) {
            break;
        }
        if (no_progress >= 8) {
            break;
        }
        vTaskDelay(pdMS_TO_TICKS(1));
    }
    return *bytes_left > 0;
}

int32_t st77916_radio_http_mp3_play(const char *url, const char *station_name, uint32_t volume_percent) {
    printf("radio-r36-r32: stream_pacing=STREAMBUFFER_PSRAM_PRODUCER_CONSUMER_R11_R1 stream_bytes=%d decode_input=%d prefill=%d decode_low_water=%d decode_target=%d http_chunk=%d blocking=YES spin=NO ring_quarantine=YES volume_quiet=YES i2s_stereo=YES mono_upmix=YES psram_stream=YES atomics=YES audio=PCM5101_I2S\n",
           RADIO_R11_STREAM_BYTES, RADIO_R11_DECODE_INPUT_BYTES, RADIO_R11_PREFILL_BYTES,
           RADIO_R11_DECODE_LOW_WATER_BYTES, RADIO_R11_DECODE_TARGET_BYTES, RADIO_R11_HTTP_CHUNK_BYTES);
    printf("radio-r36-r20: url_kind=%s url=%s stream_tuning=streambuffer input_bytes=%d refill_chunk=%d low_water=%d audio=PCM5101_I2S\n",
           radio_r20_url_kind(url), url != NULL ? url : "(null)", RADIO_R11_STREAM_BYTES, RADIO_R11_HTTP_CHUNK_BYTES, RADIO_R11_DECODE_LOW_WATER_BYTES);

    if (url == NULL || station_name == NULL) return -10;

    atomic_store_explicit(&s_radio_stop_requested, false, memory_order_release);
    atomic_store_explicit(&s_radio_elapsed_seconds, 0, memory_order_release);
    radio_set_buffered(0);
    radio_set_status(RADIO_STATUS_CONNECTING);
    st77916_radio_http_mp3_set_volume(volume_percent);

    printf("radio-r36-r15: action=CONNECT station=%s url=%s format=%s audio=PCM5101_I2S\n", station_name, url, radio_stream_format_label(url));

    esp_http_client_config_t config = {
        .url = url,
        .crt_bundle_attach = esp_crt_bundle_attach,
        .skip_cert_common_name_check = false,
        .timeout_ms = 1000,
        .buffer_size = 4096,
        .buffer_size_tx = 1024,
    };

    esp_http_client_handle_t client = esp_http_client_init(&config);
    if (client == NULL) {
        radio_set_status(RADIO_STATUS_ERROR);
        printf("radio-r36: status=HTTP_CLIENT_INIT_FAILED station=%s audio=PCM5101_I2S\n", station_name);
        return -11;
    }

    esp_http_client_set_header(client, "Icy-MetaData", "0");
    esp_http_client_set_header(client, "Accept", "audio/mpeg,audio/*,*/*");
    esp_http_client_set_header(client, "Connection", "keep-alive");
    esp_err_t err = esp_http_client_open(client, 0);
    if (err != ESP_OK) {
        radio_set_status(RADIO_STATUS_ERROR);
        printf("radio-r36-r17: status=HTTP_OPEN_FAILED station=%s err=%d https_tls=CRT_BUNDLE note=OPEN_FAILED audio=PCM5101_I2S\n", station_name, (int)err);
        esp_http_client_cleanup(client);
        return -12;
    }

    int content_length = esp_http_client_fetch_headers(client);
    int status_code = esp_http_client_get_status_code(client);
    printf("radio-r36-r15: status=%s station=%s http_status=%d content_length=%d audio=PCM5101_I2S\n",
           strncmp(url, "https://", 8) == 0 ? "HTTPS_CONNECTED" : "HTTP_CONNECTED",
           station_name, status_code, content_length);

    int small_handled = 0;
    int32_t small_code = radio_r19_handle_small_response(client, url, station_name, volume_percent, content_length, status_code, &small_handled);
    if (small_handled) return small_code;

    HMP3Decoder decoder = MP3InitDecoder();
    if (decoder == NULL) {
        radio_set_status(RADIO_STATUS_ERROR);
        esp_http_client_close(client);
        esp_http_client_cleanup(client);
        printf("radio-r36: status=ALLOC_DECODER_FAILED station=%s audio=PCM5101_I2S\n", station_name);
        return -13;
    }

    unsigned char *input = (unsigned char *)malloc(RADIO_R11_DECODE_INPUT_BYTES);
    short *output = (short *)malloc(sizeof(short) * RADIO_HELIX_OUT_SAMPLES);
    if (input == NULL || output == NULL) {
        radio_set_status(RADIO_STATUS_ERROR);
        if (input != NULL) free(input);
        if (output != NULL) free(output);
        MP3FreeDecoder(decoder);
        esp_http_client_close(client);
        esp_http_client_cleanup(client);
        printf("radio-r36: status=ALLOC_BUFFER_FAILED station=%s input=%u output_samples=%u audio=PCM5101_I2S\n",
               station_name, (unsigned)RADIO_R11_DECODE_INPUT_BYTES, (unsigned)RADIO_HELIX_OUT_SAMPLES);
        return -14;
    }

    radio_r11_producer_ctx_t producer;
    if (!radio_r11_start_producer(&producer, client)) {
        radio_set_status(RADIO_STATUS_ERROR);
        free(output);
        free(input);
        MP3FreeDecoder(decoder);
        esp_http_client_close(client);
        esp_http_client_cleanup(client);
        printf("radio-r36-r31: status=PRODUCER_START_FAILED station=%s audio=PCM5101_I2S\n", station_name);
        return -15;
    }

    unsigned char *read_ptr = input;
    int bytes_left = 0;
    bool pcm_started = false;
    bool first_write = false;
    int current_sample_rate = 0;
    int current_channels = 0;
    int decode_errors = 0;
    int frame_count = 0;
    uint64_t decoded_pcm_frames = 0;

    radio_set_status(RADIO_STATUS_BUFFERING);
    int r11_prefill_target = RADIO_R11_PREFILL_BYTES;
    if ((uint32_t)r11_prefill_target > producer.stream_bytes) {
        r11_prefill_target = (int)(producer.stream_bytes * 3U / 4U);
    }
    if (r11_prefill_target < RADIO_R11_MIN_START_BYTES) {
        r11_prefill_target = RADIO_R11_MIN_START_BYTES;
    }
    printf("radio-r36: status=BUFFERING station=%s buffer_target=%u stream_bytes=%u audio=PCM5101_I2S\n",
           station_name, (unsigned)r11_prefill_target, (unsigned)producer.stream_bytes);
    bool prefilled = radio_r11_wait_prefill(&producer, r11_prefill_target, RADIO_R11_PREFILL_WAIT_MS);
    printf("radio-r36-r32: status=PREFILL_DONE station=%s buffered=%u target=%d prefilled=%s producer_done=%s audio=PCM5101_I2S\n",
           station_name, (unsigned)radio_r11_stream_available(&producer), r11_prefill_target,
           prefilled ? "YES" : "NO",
           atomic_load_explicit(&producer.done, memory_order_acquire) ? "YES" : "NO");

    if (!prefilled && radio_r11_stream_available(&producer) < RADIO_R11_MIN_START_BYTES) {
        printf("radio-r36-r31: status=PREFILL_TIMEOUT station=%s buffered=%u action=STOP audio=PCM5101_I2S\n",
               station_name, (unsigned)radio_r11_stream_available(&producer));
        radio_set_status(RADIO_STATUS_ERROR);
        goto cleanup;
    }

    radio_r11_refill_decode_input(&producer, input, &read_ptr, &bytes_left,
                                  RADIO_R11_DECODE_TARGET_BYTES, RADIO_R11_STREAM_RECV_WAIT_MS);

    while (!radio_stop_requested()) {
        if (bytes_left < RADIO_R11_DECODE_LOW_WATER_BYTES) {
            if (!radio_r11_refill_decode_input(&producer, input, &read_ptr, &bytes_left,
                                               RADIO_R11_DECODE_TARGET_BYTES, RADIO_R11_STREAM_RECV_WAIT_MS)) {
                printf("radio-r36-r31: status=REFILL_STARVED station=%s decode=%d stream=%u producer_done=%s action=END_STREAM audio=PCM5101_I2S\n",
                       station_name, bytes_left, (unsigned)radio_r11_stream_available(&producer),
                       atomic_load_explicit(&producer.done, memory_order_acquire) ? "YES" : "NO");
                break;
            }
        }

        if (bytes_left < RADIO_R11_MIN_DECODE_BYTES) {
            radio_r11_refill_decode_input(&producer, input, &read_ptr, &bytes_left,
                                          RADIO_R11_DECODE_TARGET_BYTES, RADIO_R11_STREAM_RECV_WAIT_MS);
            if (bytes_left < RADIO_R11_MIN_DECODE_BYTES) {
                radio_r20_yield();
                continue;
            }
        }

        int sync_offset = MP3FindSyncWord(read_ptr, bytes_left);
        if (sync_offset < 0) {
            if (content_length > 0 && content_length <= RADIO_SMALL_RESPONSE_BYTES && bytes_left < RADIO_R11_MIN_DECODE_BYTES) {
                printf("radio-r36-r19: status=NO_MP3_SYNC_SMALL_RESPONSE station=%s bytes=%d content_length=%d action=STOP_NO_WDT audio=PCM5101_I2S\n", station_name, bytes_left, content_length);
                break;
            }
            if (bytes_left > 3) {
                memmove(input, read_ptr + bytes_left - 3, 3);
                read_ptr = input;
                bytes_left = 3;
            } else {
                compact_input(input, &read_ptr, &bytes_left);
            }
            if (!radio_r11_refill_decode_input(&producer, input, &read_ptr, &bytes_left,
                                               RADIO_R11_DECODE_TARGET_BYTES, RADIO_R11_STREAM_RECV_WAIT_MS)) {
                break;
            }
            continue;
        }

        read_ptr += sync_offset;
        bytes_left -= sync_offset;

        unsigned char *decode_ptr = read_ptr;
        int decode_left = bytes_left;
        int dec = MP3Decode(decoder, &decode_ptr, &decode_left, output, 0);
        int consumed = bytes_left - decode_left;

        if (consumed > 0) {
            read_ptr = decode_ptr;
            bytes_left = decode_left;
        } else if (bytes_left > 0) {
            read_ptr += 1;
            bytes_left -= 1;
        }
        radio_r11_set_buffered_total(&producer, bytes_left);

        if (dec == ERR_MP3_MAINDATA_UNDERFLOW || dec == ERR_MP3_INDATA_UNDERFLOW) {
            radio_r11_refill_decode_input(&producer, input, &read_ptr, &bytes_left,
                                          RADIO_R11_DECODE_TARGET_BYTES, RADIO_R11_STREAM_RECV_WAIT_MS);
            continue;
        }

        if (dec != ERR_MP3_NONE) {
            decode_errors++;
            if (decode_errors > RADIO_MAX_DECODE_ERRORS) {
                printf("radio-r36: status=DECODE_ERROR station=%s err=%d errors=%d audio=PCM5101_I2S\n", station_name, dec, decode_errors);
                break;
            }
            continue;
        }
        decode_errors = 0;

        MP3FrameInfo info;
        memset(&info, 0, sizeof(info));
        MP3GetLastFrameInfo(decoder, &info);
        if (info.samprate < 8000 || info.samprate > 96000 || info.nChans < 1 || info.nChans > 2 || info.bitsPerSample != 16 || info.outputSamps <= 0) {
            printf("radio-r36: status=UNSUPPORTED_FRAME station=%s sample_rate=%d channels=%d bits=%d output_samps=%d audio=PCM5101_I2S\n",
                   station_name, info.samprate, info.nChans, info.bitsPerSample, info.outputSamps);
            break;
        }

        const int radio_output_channels = 2;
        const int radio_source_channels = info.nChans;

        if (!pcm_started || current_sample_rate != info.samprate || current_channels != radio_output_channels) {
            if (pcm_started) st77916_audio_pcm_stop();
            if (!st77916_audio_pcm_init((uint32_t)info.samprate, 16, (uint16_t)radio_output_channels)) {
                printf("radio-r36: status=PCM_INIT_FAILED station=%s sample_rate=%d channels=%d source_channels=%d audio=PCM5101_I2S\n",
                       station_name, info.samprate, radio_output_channels, radio_source_channels);
                break;
            }
            pcm_started = true;
            current_sample_rate = info.samprate;
            current_channels = radio_output_channels;
            radio_set_status(RADIO_STATUS_PLAYING);
            printf("radio-r36: status=PLAYING station=%s decoder=HELIX_FIXED_POINT sample_rate=%d channels=%d source_channels=%d bitrate=%d i2s_stereo=YES mono_upmix=YES audio=PCM5101_I2S\n",
                   station_name, info.samprate, radio_output_channels, radio_source_channels, info.bitrate);
        }

        int write_samps = radio_r10_r2_prepare_i2s_stereo(output, info.outputSamps, radio_source_channels);
        scale_pcm16(output, write_samps, radio_volume_percent());
        int32_t written = st77916_audio_pcm_write((const uint8_t *)output, (uint32_t)(write_samps * (int)sizeof(short)), 250);
        if (written <= 0) {
            printf("radio-r36: status=WRITE_FAILED station=%s err=%d audio=PCM5101_I2S\n", station_name, (int)written);
            break;
        }

        if (info.samprate > 0) {
            decoded_pcm_frames += (uint64_t)(write_samps / radio_output_channels);
            atomic_store_explicit(&s_radio_elapsed_seconds, (unsigned)(decoded_pcm_frames / (uint64_t)info.samprate), memory_order_release);
        }

        frame_count++;
        if (!first_write) {
            first_write = true;
            printf("radio-r36: status=FIRST_WRITE_OK station=%s bytes=%d sample_rate=%d channels=%d source_channels=%d i2s_stereo=YES audio=PCM5101_I2S\n", station_name, (int)written, info.samprate, current_channels, radio_source_channels);
        }

        if ((frame_count % 600) == 0) {
            printf("radio-r36-r31: status=PROGRESS station=%s frames=%d elapsed_s=%u buffered=%u stream=%u decode=%d producer_done=%s volume=%u audio=PCM5101_I2S\n",
                   station_name, frame_count,
                   atomic_load_explicit(&s_radio_elapsed_seconds, memory_order_acquire),
                   atomic_load_explicit(&s_radio_buffered_bytes, memory_order_acquire),
                   (unsigned)radio_r11_stream_available(&producer), bytes_left,
                   atomic_load_explicit(&producer.done, memory_order_acquire) ? "YES" : "NO",
                   radio_volume_percent());
        }
    }

cleanup:
    if (pcm_started) st77916_audio_pcm_stop();
    radio_r11_stop_producer(&producer);
    free(output);
    free(input);
    MP3FreeDecoder(decoder);
    esp_http_client_close(client);
    esp_http_client_cleanup(client);

    int stopped = radio_stop_requested() ? 1 : 0;
    atomic_store_explicit(&s_radio_stop_requested, false, memory_order_release);
    radio_set_status(RADIO_STATUS_STOPPED);
    radio_set_buffered(0);
    printf("radio-r36: status=%s station=%s frames=%d first_write=%s audio=PCM5101_I2S\n",
           stopped ? "STOPPED" : "ENDED_OR_ERROR", station_name, frame_count, first_write ? "YES" : "NO");
    return stopped ? 1 : 0;
}

// RAW-V1-0-1-R10-RADIO-RING-QUARANTINE-DIRECT-ATOMICS

// RAW-V1-0-1-R10-R1-RADIO-VOLUME-QUIET-BUFFER-REPAIR

// RAW-V1-0-1-R10-R2-RADIO-I2S-STEREO-OUTPUT-REPAIR



// RAW-V1-0-1-R11-STREAMBUFFER-PRODUCER-CONSUMER-REPAIR


// RAW-V1-0-1-R11-R1-STREAMBUFFER-PSRAM-PRODUCER-START-REPAIR

/* RAW-V1-0-1-R11-R2-RADIO-LIVE-UI-REFRESH-C-SHIM-NO-RUNTIME-CHANGE */

// RAW-V1-0-1-R14-CLEAN-RADIO-STREAMBUFFER-SHIM
