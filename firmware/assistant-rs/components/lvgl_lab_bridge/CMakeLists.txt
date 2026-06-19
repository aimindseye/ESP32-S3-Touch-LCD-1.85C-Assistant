idf_component_register(
    SRCS "lvgl_lab_bridge.c"
    INCLUDE_DIRS "include"
)

target_compile_definitions(${COMPONENT_LIB} PRIVATE LVGL_LAB_STUB_RENDERER=1)
