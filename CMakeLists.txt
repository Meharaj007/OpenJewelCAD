cmake_minimum_required(VERSION 3.25)
project(OpenJewelCADKernel LANGUAGES CXX)
include(CTest)

set(CMAKE_CXX_STANDARD 17)
set(CMAKE_CXX_STANDARD_REQUIRED ON)
set(CMAKE_CXX_EXTENSIONS OFF)

find_package(OpenCASCADE CONFIG REQUIRED)

add_library(openjewelcad_kernel STATIC
    cpp/cad-kernel/src/cad_kernel.cpp
)
target_include_directories(openjewelcad_kernel
    PUBLIC include
    PRIVATE ${OpenCASCADE_INCLUDE_DIR}
)
target_link_libraries(openjewelcad_kernel PUBLIC ${OpenCASCADE_LIBRARIES})
target_compile_definitions(openjewelcad_kernel PRIVATE _USE_MATH_DEFINES)

add_executable(kernel_smoke apps/kernel-smoke/main.cpp)
target_link_libraries(kernel_smoke PRIVATE openjewelcad_kernel)
add_test(NAME kernel_smoke COMMAND kernel_smoke)
