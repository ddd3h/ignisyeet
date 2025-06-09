cmake_minimum_required(VERSION 3.12)
project(IgnisYeet VERSION 1.0.0 LANGUAGES CXX)

# Set C++ standard
set(CMAKE_CXX_STANDARD 17)
set(CMAKE_CXX_STANDARD_REQUIRED ON)
set(CMAKE_CXX_EXTENSIONS OFF)

# Compiler flags
set(CMAKE_CXX_FLAGS "${CMAKE_CXX_FLAGS} -Wall -Wextra")
set(CMAKE_CXX_FLAGS_DEBUG "-g -O0")
set(CMAKE_CXX_FLAGS_RELEASE "-O3 -DNDEBUG")

# Include directories
include_directories(include)

# Source files
set(PHYSICS_SOURCES
    src/physics/vector3d.cpp
    src/physics/state.cpp
)

set(ROCKET_SOURCES
    # src/rocket/vehicle.cpp  # To be implemented
)

set(ENVIRONMENT_SOURCES
    # src/environment/atmosphere.cpp  # To be implemented
)

set(SIMULATION_SOURCES
    # src/simulation/integrator.cpp  # To be implemented
    # src/simulation/engine.cpp      # To be implemented
)

set(OUTPUT_SOURCES
    # src/output/manager.cpp  # To be implemented
)

set(LEGACY_SOURCES
    src/parameter.cpp
    src/rocket.cpp
    src/output.cpp
)

# Create physics library
add_library(ignisyeet_physics ${PHYSICS_SOURCES})

# Create legacy library (for backward compatibility)
add_library(ignisyeet_legacy ${LEGACY_SOURCES})
target_link_libraries(ignisyeet_legacy ignisyeet_physics)

# Main executable (legacy)
add_executable(ignisyeet src/main.cpp)
target_link_libraries(ignisyeet ignisyeet_legacy)

# Architecture test executable
add_executable(test_architecture test_architecture.cpp)
target_link_libraries(test_architecture ignisyeet_physics)

# Test target
enable_testing()
add_test(NAME architecture_test COMMAND test_architecture)

# Custom target for running tests
add_custom_target(test-arch
    COMMAND test_architecture
    DEPENDS test_architecture
    COMMENT "Running architecture tests"
)

# Installation
install(TARGETS ignisyeet DESTINATION bin)
install(DIRECTORY include/ DESTINATION include/ignisyeet)

# Package configuration
include(CMakePackageConfigHelpers)
configure_package_config_file(
    "${CMAKE_CURRENT_SOURCE_DIR}/cmake/IgnisYeetConfig.cmake.in"
    "${CMAKE_CURRENT_BINARY_DIR}/IgnisYeetConfig.cmake"
    INSTALL_DESTINATION lib/cmake/IgnisYeet
)

write_basic_package_version_file(
    "${CMAKE_CURRENT_BINARY_DIR}/IgnisYeetConfigVersion.cmake"
    VERSION ${PROJECT_VERSION}
    COMPATIBILITY SameMajorVersion
)

install(FILES
    "${CMAKE_CURRENT_BINARY_DIR}/IgnisYeetConfig.cmake"
    "${CMAKE_CURRENT_BINARY_DIR}/IgnisYeetConfigVersion.cmake"
    DESTINATION lib/cmake/IgnisYeet
)

# Export targets
install(TARGETS ignisyeet_physics ignisyeet_legacy
    EXPORT IgnisYeetTargets
    LIBRARY DESTINATION lib
    ARCHIVE DESTINATION lib
    RUNTIME DESTINATION bin
)

install(EXPORT IgnisYeetTargets
    FILE IgnisYeetTargets.cmake
    NAMESPACE IgnisYeet::
    DESTINATION lib/cmake/IgnisYeet
)

# Documentation
find_package(Doxygen QUIET)
if(DOXYGEN_FOUND)
    set(DOXYGEN_IN ${CMAKE_CURRENT_SOURCE_DIR}/doc/Doxyfile.in)
    set(DOXYGEN_OUT ${CMAKE_CURRENT_BINARY_DIR}/Doxyfile)
    
    if(EXISTS ${DOXYGEN_IN})
        configure_file(${DOXYGEN_IN} ${DOXYGEN_OUT} @ONLY)
        
        add_custom_target(doc_doxygen ALL
            COMMAND ${DOXYGEN_EXECUTABLE} ${DOXYGEN_OUT}
            WORKING_DIRECTORY ${CMAKE_CURRENT_BINARY_DIR}
            COMMENT "Generating API documentation with Doxygen"
            VERBATIM
        )
    endif()
endif()

# Print configuration summary
message(STATUS "IgnisYeet Configuration Summary:")
message(STATUS "  Version: ${PROJECT_VERSION}")
message(STATUS "  Build type: ${CMAKE_BUILD_TYPE}")
message(STATUS "  C++ standard: ${CMAKE_CXX_STANDARD}")
message(STATUS "  Install prefix: ${CMAKE_INSTALL_PREFIX}")
message(STATUS "  Physics library: enabled")
message(STATUS "  Legacy support: enabled")
message(STATUS "  Tests: enabled")
if(DOXYGEN_FOUND)
    message(STATUS "  Documentation: enabled")
else()
    message(STATUS "  Documentation: disabled (Doxygen not found)")
endif()
