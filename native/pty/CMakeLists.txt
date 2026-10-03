cmake_minimum_required(VERSION 3.22)
project(workflow_runtime LANGUAGES C CXX)

add_library(workflow_pty SHARED pty_process.c pty_jni.cpp)
target_compile_features(workflow_pty PRIVATE c_std_11 cxx_std_17)
target_compile_options(workflow_pty PRIVATE -Wall -Wextra -Werror -fvisibility=hidden)
# Keep load segments compatible with Android devices using 16 KiB pages.
target_link_options(workflow_pty PRIVATE -Wl,-z,max-page-size=16384 -Wl,-z,common-page-size=16384)
