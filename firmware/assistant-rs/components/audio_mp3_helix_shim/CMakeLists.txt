idf_component_register(SRCS "audio_mp3_helix_shim.c"
                       INCLUDE_DIRS "."
                       REQUIRES chmorgan__esp-libhelix-mp3 audio_pcm_shim)
