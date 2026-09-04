cmake_minimum_required(VERSION 3.22.1)
project(ympresence LANGUAGES CXX)

find_package(discord_partner_sdk REQUIRED CONFIG)

add_library(ympresence SHARED
        ../../../../../app/src/main/cpp/presence.cpp)

find_library(log-lib log)

target_link_libraries(ympresence
        discord_partner_sdk::discord_partner_sdk
        ${log-lib})
