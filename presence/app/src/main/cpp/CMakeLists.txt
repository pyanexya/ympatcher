cmake_minimum_required(VERSION 3.22.1)
project(ympresence LANGUAGES CXX)

find_package(discord_partner_sdk REQUIRED CONFIG)

add_library(ympresence SHARED presence.cpp)

target_compile_features(ympresence PRIVATE cxx_std_17)
target_compile_options(ympresence PRIVATE -Wall -Wextra -Werror)

target_link_libraries(
    ympresence
    PRIVATE
    discord_partner_sdk::discord_partner_sdk
    android
    log)
