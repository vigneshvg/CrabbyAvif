# Copyright 2024 Google LLC
#
# Licensed under the Apache License, Version 2.0 (the "License");
# you may not use this file except in compliance with the License.
# You may obtain a copy of the License at
#
#     http://www.apache.org/licenses/LICENSE-2.0
#
# Unless required by applicable law or agreed to in writing, software
# distributed under the License is distributed on an "AS IS" BASIS,
# WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
# See the License for the specific language governing permissions and
# limitations under the License.

cmake_minimum_required(VERSION 3.10)

project(crabby_avif_c_api_tests)

enable_testing()

set(CMAKE_C_COMPILER "clang")
set(CMAKE_CXX_COMPILER "clang++")

cmake_path(GET CMAKE_CURRENT_SOURCE_DIR PARENT_PATH CARGO_ROOT_DIR)

set(GTEST_INCLUDE_DIR "${CARGO_ROOT_DIR}/external/googletest/googletest/include")
set(GTEST_LIBRARIES "${CARGO_ROOT_DIR}/external/googletest/build/lib/libgtest.a")
set(GTEST_MAIN_LIBRARIES "${CARGO_ROOT_DIR}/external/googletest/build/lib/libgtest_main.a")

set(CRABBY_AVIF_INCLUDE_DIR "${CARGO_ROOT_DIR}/include")
set(CRABBY_AVIF_LIBRARIES "${CARGO_ROOT_DIR}/target/release/libcrabby_avif.so")

add_library(testutil OBJECT testutil.cc)
target_include_directories(testutil PRIVATE ${GTEST_INCLUDE_DIR})
target_include_directories(testutil PRIVATE ${CRABBY_AVIF_INCLUDE_DIR})
target_link_libraries(testutil PRIVATE ${GTEST_LIBRARIES})
target_link_libraries(testutil PRIVATE ${CRABBY_AVIF_LIBRARIES})

macro(add_avif_gtest TEST_NAME)
    add_executable(${TEST_NAME} ${TEST_NAME}.cc)
    target_include_directories(${TEST_NAME} PRIVATE ${GTEST_INCLUDE_DIR})
    target_include_directories(${TEST_NAME} PRIVATE ${CRABBY_AVIF_INCLUDE_DIR})
    target_link_libraries(${TEST_NAME} PRIVATE testutil)
    target_link_libraries(${TEST_NAME} PRIVATE ${GTEST_LIBRARIES})
    target_link_libraries(${TEST_NAME} PRIVATE ${GTEST_MAIN_LIBRARIES})
    target_link_libraries(${TEST_NAME} PRIVATE ${CRABBY_AVIF_LIBRARIES})
    add_test(NAME ${TEST_NAME} COMMAND ${TEST_NAME} ${CARGO_ROOT_DIR}/tests/data/)
endmacro()

add_avif_gtest(decoder_tests)
add_avif_gtest(encoder_tests)
add_avif_gtest(gainmap_tests)
add_avif_gtest(image_tests)
add_avif_gtest(incremental_tests)
add_avif_gtest(reformat_tests)

# Conformance test.
add_executable(conformance_tests conformance_tests.cc)
target_include_directories(conformance_tests PRIVATE ${GTEST_INCLUDE_DIR})
target_include_directories(conformance_tests PRIVATE ${CRABBY_AVIF_INCLUDE_DIR})
target_link_libraries(conformance_tests PRIVATE ${GTEST_LIBRARIES})
target_link_libraries(conformance_tests PRIVATE ${GTEST_MAIN_LIBRARIES})
target_link_libraries(conformance_tests PRIVATE ${CRABBY_AVIF_LIBRARIES})
add_test(NAME conformance_tests COMMAND conformance_tests ${CARGO_ROOT_DIR}/external/av1-avif/testFiles/)
