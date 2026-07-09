idf_component_register(
    SRCS
        "st77916_shim.c"
        "esp_lcd_st77916.c"
    INCLUDE_DIRS
        "include"
    REQUIRES esp_timer
        driver
        esp_lcd
        fatfs
        sdmmc
        esp_http_client
        esp_adc
)