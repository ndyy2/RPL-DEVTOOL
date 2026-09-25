cmake_minimum_required(VERSION 3.16)
project(rplkit VERSION 0.2.0 LANGUAGES CXX)

set(CMAKE_CXX_STANDARD 17)
set(CMAKE_CXX_STANDARD_REQUIRED ON)
set(CMAKE_CXX_EXTENSIONS OFF)

add_executable(rplkit
  src/main.cpp
  src/runtime.cpp
  src/module_manager.cpp
  src/system_services.cpp
  src/native_tools.cpp
)
target_include_directories(rplkit PRIVATE include)

if(MSVC)
  target_compile_options(rplkit PRIVATE /W4)
else()
  target_compile_options(rplkit PRIVATE -Wall -Wextra -Wpedantic)
endif()
