cmake_minimum_required(VERSION 3.16)
project(rplkit VERSION 0.3.0 LANGUAGES CXX)

set(CMAKE_CXX_STANDARD 17)
set(CMAKE_CXX_STANDARD_REQUIRED ON)
set(CMAKE_CXX_EXTENSIONS OFF)

# --- Rust runtime (staticlib). Release diutamakan, debug sebagai fallback.
set(RUST_LIB_RELEASE "${CMAKE_SOURCE_DIR}/../../core/rust/target/release/librplkit_runtime.a")
set(RUST_LIB_DEBUG "${CMAKE_SOURCE_DIR}/../../core/rust/target/debug/librplkit_runtime.a")
if(EXISTS "${RUST_LIB_RELEASE}")
  set(RUST_LIB "${RUST_LIB_RELEASE}")
elseif(EXISTS "${RUST_LIB_DEBUG}")
  set(RUST_LIB "${RUST_LIB_DEBUG}")
else()
  message(FATAL_ERROR
    "librplkit_runtime.a tidak ditemukan.\n"
    "Bangun dulu: cargo build --manifest-path ${CMAKE_SOURCE_DIR}/../../core/rust/Cargo.toml")
endif()
message(STATUS "Rust runtime: ${RUST_LIB}")

# --- Flag link embed Python (dibutuhkan PyO3 di dalam staticlib Rust).
find_program(PYTHON3 python3 REQUIRED)
execute_process(
  COMMAND ${PYTHON3}-config --ldflags --embed
  OUTPUT_VARIABLE PY_LDFLAGS OUTPUT_STRIP_TRAILING_WHITESPACE
  RESULT_VARIABLE PY_RES
)
if(NOT PY_RES EQUAL 0)
  message(FATAL_ERROR "python3-config --ldflags --embed gagal")
endif()
separate_arguments(PY_LDFLAGS_LIST UNIX_COMMAND "${PY_LDFLAGS}")

add_executable(rplkit
  src/main.cpp
  src/runtime.cpp
  src/module_manager.cpp
  src/system_services.cpp
)
target_include_directories(rplkit PRIVATE include)
target_link_libraries(rplkit PRIVATE ${RUST_LIB} ${PY_LDFLAGS_LIST} pthread dl m)

if(MSVC)
  target_compile_options(rplkit PRIVATE /W4)
else()
  target_compile_options(rplkit PRIVATE -Wall -Wextra -Wpedantic)
endif()
