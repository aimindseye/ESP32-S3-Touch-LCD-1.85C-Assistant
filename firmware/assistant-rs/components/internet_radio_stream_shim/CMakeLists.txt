idf_component_register(SRCS "internet_radio_stream_shim.c"
                       INCLUDE_DIRS "."
                       REQUIRES esp_http_client chmorgan__esp-libhelix-mp3 audio_pcm_shim mbedtls esp-tls)
