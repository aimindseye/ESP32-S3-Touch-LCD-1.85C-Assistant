#include <stdbool.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include "mp3dec.h"

extern bool st77916_audio_pcm_init(uint32_t sample_rate, uint16_t bits_per_sample, uint16_t channels);
extern int32_t st77916_audio_pcm_write(const uint8_t *data, uint32_t len, uint32_t timeout_ms);
extern void st77916_audio_pcm_stop(void);

#define MP3_HELIX_INPUT_BYTES 8192
#define MP3_HELIX_OUT_SAMPLES (MAX_NCHAN * MAX_NGRAN * MAX_NSAMP)
#define MP3_HELIX_MAX_DECODE_ERRORS 64

static volatile bool s_mp3_stop_requested = false;
static volatile uint32_t s_mp3_volume_percent = 60;
static volatile uint32_t s_mp3_progress_percent = 0;
static volatile uint32_t s_mp3_elapsed_seconds = 0;
static volatile uint32_t s_mp3_duration_seconds = 0;

static void reset_runtime_progress(void) {
    s_mp3_progress_percent = 0;
    s_mp3_elapsed_seconds = 0;
    s_mp3_duration_seconds = 0;
}

void st77916_audio_mp3_helix_set_volume(uint32_t volume_percent) {
    s_mp3_volume_percent = volume_percent > 100 ? 100 : volume_percent;
}

uint32_t st77916_audio_mp3_helix_progress_percent(void) {
    return s_mp3_progress_percent;
}

uint32_t st77916_audio_mp3_helix_elapsed_seconds(void) {
    return s_mp3_elapsed_seconds;
}

uint32_t st77916_audio_mp3_helix_duration_seconds(void) {
    return s_mp3_duration_seconds;
}


void st77916_audio_mp3_helix_stop_request(void) {
    s_mp3_stop_requested = true;
    printf("audio-mp3-helix-r35-r2: stop_request=DEFERRED_TO_DECODE_THREAD i2s_stop_owner=DECODE_THREAD concurrent_i2s_disable_guard=YES audio=PCM5101_I2S\n");
}



static bool refill_input(FILE *file, unsigned char *input, unsigned char **read_ptr, int *bytes_left, bool *eof) {
    if (*read_ptr != input && *bytes_left > 0) {
        memmove(input, *read_ptr, (size_t)*bytes_left);
    }
    *read_ptr = input;
    if (*eof) {
        return *bytes_left > 0;
    }
    int space = MP3_HELIX_INPUT_BYTES - *bytes_left;
    if (space <= 0) {
        return true;
    }
    size_t n = fread(input + *bytes_left, 1, (size_t)space, file);
    if (n == 0) {
        *eof = true;
    } else {
        *bytes_left += (int)n;
    }
    return *bytes_left > 0;
}

static void scale_pcm16(short *samples, int sample_count, uint32_t volume_percent) {
    int32_t volume = (int32_t)(volume_percent > 100 ? 100 : volume_percent);
    for (int i = 0; i < sample_count; i++) {
        int32_t scaled = ((int32_t)samples[i] * volume) / 100;
        if (scaled > 32767) {
            scaled = 32767;
        } else if (scaled < -32768) {
            scaled = -32768;
        }
        samples[i] = (short)scaled;
    }
}


int32_t st77916_audio_mp3_helix_play_file(const char *path, uint32_t volume_percent) {
    if (path == NULL || path[0] == '\0') {
        return -10;
    }
    FILE *file = fopen(path, "rb");
    if (file == NULL) {
        printf("audio-mp3-helix-r35-r1: status=OPEN_FAILED path=%s audio=PCM5101_I2S\n", path);
        return -11;
    }

    long file_size = 0;
    if (fseek(file, 0, SEEK_END) == 0) {
        long end_pos = ftell(file);
        if (end_pos > 0) {
            file_size = end_pos;
        }
        fseek(file, 0, SEEK_SET);
    }

    HMP3Decoder decoder = MP3InitDecoder();
    if (decoder == NULL) {
        fclose(file);
        printf("audio-mp3-helix-r35-r1: status=ALLOC_DECODER_FAILED path=%s audio=PCM5101_I2S\n", path);
        return -12;
    }
    unsigned char *input = (unsigned char *)malloc(MP3_HELIX_INPUT_BYTES);
    short *output = (short *)malloc(sizeof(short) * MP3_HELIX_OUT_SAMPLES);
    if (input == NULL || output == NULL) {
        if (input != NULL) free(input);
        if (output != NULL) free(output);
        MP3FreeDecoder(decoder);
        fclose(file);
        printf("audio-mp3-helix-r35-r1: status=ALLOC_BUFFER_FAILED input=%u out_samples=%u audio=PCM5101_I2S\n", (unsigned)MP3_HELIX_INPUT_BYTES, (unsigned)MP3_HELIX_OUT_SAMPLES);
        return -13;
    }

    s_mp3_stop_requested = false;
    st77916_audio_mp3_helix_set_volume(volume_percent);
    reset_runtime_progress();

    unsigned char *read_ptr = input;
    int bytes_left = 0;
    bool eof = false;
    bool pcm_started = false;
    bool first_write = false;
    int current_sample_rate = 0;
    int current_channels = 0;
    int decode_errors = 0;
    int frame_count = 0;
    uint64_t decoded_pcm_frames = 0;

    printf("audio-mp3-helix-r35-r1: status=DECODE_STARTED path=%s input_buffer=%u output_samples=%u audio=PCM5101_I2S\n", path, (unsigned)MP3_HELIX_INPUT_BYTES, (unsigned)MP3_HELIX_OUT_SAMPLES);
    while (!s_mp3_stop_requested) {
        if (bytes_left < 2048 && !eof) {
            refill_input(file, input, &read_ptr, &bytes_left, &eof);
        }
        if (bytes_left <= 0) {
            break;
        }

        int sync_offset = MP3FindSyncWord(read_ptr, bytes_left);
        if (sync_offset < 0) {
            if (eof) {
                break;
            }
            if (bytes_left > 3) {
                memmove(input, read_ptr + bytes_left - 3, 3);
                read_ptr = input;
                bytes_left = 3;
            } else if (read_ptr != input && bytes_left > 0) {
                memmove(input, read_ptr, (size_t)bytes_left);
                read_ptr = input;
            }
            refill_input(file, input, &read_ptr, &bytes_left, &eof);
            continue;
        }

        read_ptr += sync_offset;
        bytes_left -= sync_offset;
        unsigned char *decode_ptr = read_ptr;
        int decode_left = bytes_left;
        int err = MP3Decode(decoder, &decode_ptr, &decode_left, output, 0);
        int consumed = bytes_left - decode_left;

        if (consumed > 0) {
            read_ptr = decode_ptr;
            bytes_left = decode_left;
        } else if (bytes_left > 0) {
            read_ptr += 1;
            bytes_left -= 1;
        }

        if (err == ERR_MP3_MAINDATA_UNDERFLOW || err == ERR_MP3_INDATA_UNDERFLOW) {
            if (!eof) {
                refill_input(file, input, &read_ptr, &bytes_left, &eof);
                continue;
            }
        }
        if (err != ERR_MP3_NONE) {
            decode_errors++;
            if (decode_errors > MP3_HELIX_MAX_DECODE_ERRORS) {
                printf("audio-mp3-helix-r35-r1: status=DECODE_ERROR err=%d errors=%d audio=PCM5101_I2S\n", err, decode_errors);
                break;
            }
            continue;
        }

        MP3FrameInfo info;
        memset(&info, 0, sizeof(info));
        MP3GetLastFrameInfo(decoder, &info);
        if (info.samprate < 8000 || info.samprate > 96000 || info.nChans < 1 || info.nChans > 2 || info.bitsPerSample != 16 || info.outputSamps <= 0) {
            printf("audio-mp3-helix-r35-r1: status=UNSUPPORTED_FRAME sample_rate=%d channels=%d bits=%d output_samps=%d audio=PCM5101_I2S\n", info.samprate, info.nChans, info.bitsPerSample, info.outputSamps);
            break;
        }

        if (!pcm_started || current_sample_rate != info.samprate || current_channels != info.nChans) {
            if (pcm_started) {
                st77916_audio_pcm_stop();
            }
            if (!st77916_audio_pcm_init((uint32_t)info.samprate, 16, (uint16_t)info.nChans)) {
                printf("audio-mp3-helix-r35-r1: status=PCM_INIT_FAILED sample_rate=%d channels=%d audio=PCM5101_I2S\n", info.samprate, info.nChans);
                break;
            }
            pcm_started = true;
            current_sample_rate = info.samprate;
            current_channels = info.nChans;
            printf("audio-mp3-helix-r35-r1: status=PCM_STARTED sample_rate=%d channels=%d bitrate=%d layer=%d version=%d audio=PCM5101_I2S\n", info.samprate, info.nChans, info.bitrate, info.layer, info.version);
        }

        scale_pcm16(output, info.outputSamps, s_mp3_volume_percent);
        int32_t written = st77916_audio_pcm_write((const uint8_t *)output, (uint32_t)(info.outputSamps * (int)sizeof(short)), 250);
        if (written <= 0) {
            printf("audio-mp3-helix-r35-r1: status=WRITE_FAILED err=%d audio=PCM5101_I2S\n", (int)written);
            break;
        }

        if (info.nChans > 0 && info.samprate > 0) {
            decoded_pcm_frames += (uint64_t)(info.outputSamps / info.nChans);
            uint32_t elapsed = (uint32_t)(decoded_pcm_frames / (uint64_t)info.samprate);
            s_mp3_elapsed_seconds = elapsed;

            long current_pos = ftell(file);
            long approx_consumed = current_pos >= 0 ? current_pos - bytes_left : 0;
            if (approx_consumed < 0) {
                approx_consumed = 0;
            }
            if (file_size > 0) {
                uint32_t percent = (uint32_t)(((uint64_t)approx_consumed * 100ULL) / (uint64_t)file_size);
                if (percent > 100) {
                    percent = 100;
                }
                s_mp3_progress_percent = percent;
                if (percent > 0) {
                    uint32_t duration = (uint32_t)(((uint64_t)elapsed * 100ULL) / (uint64_t)percent);
                    if (duration < elapsed) {
                        duration = elapsed;
                    }
                    s_mp3_duration_seconds = duration;
                }
            }
        }

        frame_count++;
        if (!first_write) {
            first_write = true;
            printf("audio-mp3-helix-r35-r1: status=FIRST_WRITE_OK bytes=%d sample_rate=%d channels=%d audio=PCM5101_I2S\n", (int)written, info.samprate, info.nChans);
        }
        if ((frame_count % 100) == 0) {
            printf("audio-mp3-helix-r35-r1: status=PROGRESS frames=%d sample_rate=%d channels=%d audio=PCM5101_I2S\n", frame_count, current_sample_rate, current_channels);
        }
        if ((frame_count % 150) == 0) {
            printf("audio-mp3-helix-r35-r3: status=UI_PROGRESS percent=%u elapsed_s=%u duration_s=%u volume=%u audio=PCM5101_I2S\n",
                   (unsigned)s_mp3_progress_percent,
                   (unsigned)s_mp3_elapsed_seconds,
                   (unsigned)s_mp3_duration_seconds,
                   (unsigned)s_mp3_volume_percent);
        }
    }

    if (!s_mp3_stop_requested) {
        s_mp3_progress_percent = 100;
        if (s_mp3_duration_seconds < s_mp3_elapsed_seconds) {
            s_mp3_duration_seconds = s_mp3_elapsed_seconds;
        }
    }

    if (pcm_started) {
        st77916_audio_pcm_stop();
    }
    free(output);
    free(input);
    MP3FreeDecoder(decoder);
    fclose(file);

    int32_t status = s_mp3_stop_requested ? 1 : 0;
    printf("audio-mp3-helix-r35-r1: status=%s frames=%d first_write=%s audio=PCM5101_I2S\n", s_mp3_stop_requested ? "STOPPED" : "COMPLETE", frame_count, first_write ? "YES" : "NO");
    s_mp3_stop_requested = false;
    return status;
}

